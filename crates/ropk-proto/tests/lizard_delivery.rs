// Contract for ropk_proto::lizard::delivery -- DESIRED vs DELIVERED for the three
// lizard reports on one shared HID endpoint. Fixes C's stuck-input bug: rfLizard()
// updated its last-sent copy (pmbtn / pkc / prevCC) even when ready() was false,
// so a dropped release left the key held forever.
//
// Rules:
//  * mouse buttons, keyboard, consumer are STATE: pending while delivered != desired
//  * mouse x/y/wheel/pan are DELTAS: accumulated (clamped to one report) until sent
//  * delivered changes ONLY on success; a FAILED send makes it UNKNOWN (the host
//    may or may not have it), so the current state is always resent -- never stuck
//  * one report in flight at a time; report kinds take turns (round robin)
//  * release_all(): desired -> neutral, motion dropped, drains until settled
//  * on_bus_reset(): host was re-enumerated -> host is neutral, re-assert desired
//
// C: mode_lizard.cpp rfLizard() report blocks + rfLizardRelease(), puck_hid.cpp handoff.
// Covers T045, T046, T047, T048, T049, and the B8 replug case.

use proptest::prelude::*;
use ropk_proto::hid::consumer::{ConsumerControls, ConsumerReport};
use ropk_proto::hid::keyboard::{KEYBOARD_REPORT_LEN, Key, KeyboardReport, Modifiers};
use ropk_proto::hid::mouse::{MouseButtons, MouseReport};
use ropk_proto::lizard::delivery::{
    Delivery, MAX_PENDING_MOTION, MAX_REPORT_LEN, OutReport, ReportKind,
};
use ropk_proto::lizard::engine::Desired;

// ---- builders ----

fn kbd(mods: Modifiers, keys: &[Key]) -> KeyboardReport {
    let mut r = KeyboardReport::NEUTRAL;
    r.modifiers = mods;
    for &k in keys {
        r.press(k);
    }
    r
}

fn with_keys(keys: &[Key]) -> Desired {
    let mut d = Desired::NEUTRAL;
    d.keyboard = kbd(Modifiers::EMPTY, keys);
    d
}

fn with_buttons(b: MouseButtons) -> Desired {
    let mut d = Desired::NEUTRAL;
    d.mouse.buttons = b;
    d
}

fn with_motion(x: i8, y: i8, wheel: i8) -> Desired {
    let mut d = Desired::NEUTRAL;
    d.mouse.x = x;
    d.mouse.y = y;
    d.mouse.wheel = wheel;
    d
}

fn with_consumer(c: ConsumerControls) -> Desired {
    let mut d = Desired::NEUTRAL;
    d.consumer = ConsumerReport { controls: c };
    d
}

fn mouse(b: MouseButtons, x: i8, y: i8, wheel: i8) -> OutReport {
    OutReport::Mouse(MouseReport { buttons: b, x, y, wheel, pan: 0 })
}

fn kb_out(keys: &[Key]) -> OutReport {
    OutReport::Keyboard(kbd(Modifiers::EMPTY, keys))
}

fn cc_out(c: ConsumerControls) -> OutReport {
    OutReport::Consumer(ConsumerReport { controls: c })
}

/// send everything pending, every send succeeds; returns what went out
fn drain_ok(d: &mut Delivery) -> Vec<OutReport> {
    let mut sent = Vec::new();
    for _ in 0..20 {
        let Some(r) = d.next() else { return sent };
        d.on_sent(true);
        sent.push(r);
    }
    panic!("delivery never settled: {sent:?}");
}

/// one attempt that succeeds / fails
fn send(d: &mut Delivery, ok: bool) -> OutReport {
    let r = d.next().expect("expected a pending report");
    d.on_sent(ok);
    r
}

fn settled_with(desired: Desired) -> Delivery {
    let mut d = Delivery::new(true);
    d.set_desired(&desired);
    drain_ok(&mut d);
    assert!(d.is_settled());
    d
}

// ---- OutReport ----

#[test]
fn max_report_len_is_the_keyboard() {
    assert_eq!(MAX_REPORT_LEN, KEYBOARD_REPORT_LEN);
    assert_eq!(MAX_REPORT_LEN, 9);
}

#[test]
fn out_report_kind() {
    assert_eq!(mouse(MouseButtons::EMPTY, 0, 0, 0).kind(), ReportKind::Mouse);
    assert_eq!(kb_out(&[]).kind(), ReportKind::Keyboard);
    assert_eq!(cc_out(ConsumerControls::EMPTY).kind(), ReportKind::Consumer);
}

#[test]
fn out_report_encode_matches_each_encoder() {
    let m = MouseReport { buttons: MouseButtons::LEFT, x: 3, y: -2, wheel: 1, pan: 0 };
    let k = kbd(Modifiers::LCTRL, &[Key::ENTER]);
    let c = ConsumerReport { controls: ConsumerControls::VOL_UP };
    let mut buf = [0u8; MAX_REPORT_LEN];

    let n = OutReport::Mouse(m).encode(&mut buf);
    assert_eq!(&buf[..n], &m.encode());
    let n = OutReport::Keyboard(k).encode(&mut buf);
    assert_eq!(&buf[..n], &k.encode());
    let n = OutReport::Consumer(c).encode(&mut buf);
    assert_eq!(&buf[..n], &c.encode());
}

// ---- construction ----

// fresh enumeration: the host holds nothing, so there is nothing to send
#[test]
fn new_is_settled() {
    let mut d = Delivery::new(true);
    assert!(d.is_settled());
    assert_eq!(d.in_flight(), None);
    assert_eq!(d.next(), None);
}

#[test]
fn new_is_const() {
    const D: Delivery = Delivery::new(true);
    assert!(D.is_settled());
}

#[test]
fn max_pending_motion_is_one_report() {
    assert_eq!(MAX_PENDING_MOTION, 127);
}

// ---- state reports: basics ----

#[test]
fn press_then_release() {
    let mut d = Delivery::new(true);
    d.set_desired(&with_keys(&[Key::ENTER]));
    assert!(d.is_pending(ReportKind::Keyboard));
    assert_eq!(send(&mut d, true), kb_out(&[Key::ENTER]));
    assert!(d.is_settled());

    d.set_desired(&Desired::NEUTRAL);
    assert_eq!(send(&mut d, true), kb_out(&[]));
    assert!(d.is_settled());
}

// unchanged state is never resent (C's change-dedup, kept)
#[test]
fn unchanged_state_is_not_resent() {
    let mut d = settled_with(with_keys(&[Key::ENTER]));
    for _ in 0..5 {
        d.set_desired(&with_keys(&[Key::ENTER]));
        assert_eq!(d.next(), None);
    }
}

// only the latest state matters: three changes before a send = one report
#[test]
fn latest_state_wins() {
    let mut d = Delivery::new(true);
    d.set_desired(&with_keys(&[Key::A]));
    d.set_desired(&with_keys(&[Key::O]));
    d.set_desired(&with_keys(&[Key::TAB]));
    assert_eq!(drain_ok(&mut d), vec![kb_out(&[Key::TAB])]);
}

#[test]
fn one_report_in_flight_at_a_time() {
    let mut d = Delivery::new(true);
    d.set_desired(&with_keys(&[Key::ENTER]));
    assert!(d.next().is_some());
    assert_eq!(d.in_flight(), Some(ReportKind::Keyboard));
    assert_eq!(d.next(), None);
    assert!(!d.is_settled());
    d.on_sent(true);
    assert_eq!(d.in_flight(), None);
}

#[test]
fn on_sent_without_flight_is_ignored() {
    let mut d = Delivery::new(true);
    d.on_sent(true);
    d.on_sent(false);
    assert!(d.is_settled());
}

// the engine keeps running while USB is busy: a change during flight is sent next
#[test]
fn change_during_flight_is_sent_after() {
    let mut d = Delivery::new(true);
    d.set_desired(&with_keys(&[Key::ENTER]));
    assert_eq!(d.next(), Some(kb_out(&[Key::ENTER])));
    d.set_desired(&Desired::NEUTRAL);
    d.on_sent(true);
    assert_eq!(drain_ok(&mut d), vec![kb_out(&[])]);
}

// ...and a change back to what's in flight needs nothing more once it lands
#[test]
fn change_back_during_flight_needs_nothing() {
    let mut d = Delivery::new(true);
    d.set_desired(&with_keys(&[Key::ENTER]));
    d.next();
    d.set_desired(&Desired::NEUTRAL);
    d.set_desired(&with_keys(&[Key::ENTER]));
    d.on_sent(true);
    assert!(d.is_settled());
}

// ---- motion ----

#[test]
fn motion_is_sent_and_then_nothing() {
    let mut d = Delivery::new(true);
    d.set_desired(&with_motion(5, -3, 1));
    assert_eq!(drain_ok(&mut d), vec![mouse(MouseButtons::EMPTY, 5, -3, 1)]);
    d.set_desired(&Desired::NEUTRAL);
    assert_eq!(d.next(), None);
}

#[test]
fn motion_accumulates_until_sent() {
    let mut d = Delivery::new(true);
    for _ in 0..3 {
        d.set_desired(&with_motion(10, -4, 1));
    }
    assert_eq!(drain_ok(&mut d), vec![mouse(MouseButtons::EMPTY, 30, -12, 3)]);
}

// a long USB stall must not end in a cursor fling: at most one report is buffered
#[test]
fn pending_motion_is_clamped_to_one_report() {
    let mut d = Delivery::new(true);
    for _ in 0..10 {
        d.set_desired(&with_motion(100, -100, 100));
    }
    assert_eq!(drain_ok(&mut d), vec![mouse(MouseButtons::EMPTY, 127, -127, 127)]);
}

#[test]
fn pan_accumulates_too() {
    let mut d = Delivery::new(true);
    let mut m = Desired::NEUTRAL;
    m.mouse.pan = 2;
    d.set_desired(&m);
    d.set_desired(&m);
    let r = drain_ok(&mut d);
    assert_eq!(r.len(), 1);
    assert!(matches!(r[0], OutReport::Mouse(MouseReport { pan: 4, .. })));
}

#[test]
fn failed_motion_is_retried() {
    let mut d = Delivery::new(true);
    d.set_desired(&with_motion(10, 0, 0));
    assert_eq!(send(&mut d, false), mouse(MouseButtons::EMPTY, 10, 0, 0));
    assert_eq!(drain_ok(&mut d), vec![mouse(MouseButtons::EMPTY, 10, 0, 0)]);
}

// motion that arrives while a mouse report is in flight is kept, not wiped
#[test]
fn motion_during_flight_is_kept() {
    let mut d = Delivery::new(true);
    d.set_desired(&with_motion(10, 0, 0));
    assert_eq!(d.next(), Some(mouse(MouseButtons::EMPTY, 10, 0, 0)));
    d.set_desired(&with_motion(5, 0, 0));
    d.on_sent(true);
    assert_eq!(drain_ok(&mut d), vec![mouse(MouseButtons::EMPTY, 5, 0, 0)]);
}

// the mouse report always carries the current buttons
#[test]
fn motion_carries_held_buttons() {
    let mut d = settled_with(with_buttons(MouseButtons::LEFT));
    let mut m = with_motion(4, 0, 0);
    m.mouse.buttons = MouseButtons::LEFT;
    d.set_desired(&m);
    assert_eq!(drain_ok(&mut d), vec![mouse(MouseButtons::LEFT, 4, 0, 0)]);
}

#[test]
fn button_change_without_motion_sends_zero_motion() {
    let mut d = Delivery::new(true);
    d.set_desired(&with_buttons(MouseButtons::MIDDLE));
    assert_eq!(drain_ok(&mut d), vec![mouse(MouseButtons::MIDDLE, 0, 0, 0)]);
}

// ---- T045: keyboard endpoint busy ----

#[test]
fn t045_press_busy_then_available_while_held() {
    let mut d = Delivery::new(true);
    d.set_desired(&with_keys(&[Key::ENTER]));
    send(&mut d, false);
    send(&mut d, false);
    assert_eq!(drain_ok(&mut d), vec![kb_out(&[Key::ENTER])]);
}

// policy: latest state wins. A tap shorter than the outage may be lost; it is never stuck.
#[test]
fn t045_press_busy_released_during_outage() {
    let mut d = Delivery::new(true);
    d.set_desired(&with_keys(&[Key::ENTER]));
    send(&mut d, false);
    d.set_desired(&Desired::NEUTRAL);
    assert_eq!(drain_ok(&mut d), vec![kb_out(&[])]);
    assert!(d.is_settled());
}

#[test]
fn t045_release_busy_then_available() {
    let mut d = settled_with(with_keys(&[Key::ENTER]));
    d.set_desired(&Desired::NEUTRAL);
    send(&mut d, false);
    send(&mut d, false);
    assert!(d.is_pending(ReportKind::Keyboard));
    assert_eq!(drain_ok(&mut d), vec![kb_out(&[])]);
}

// ---- T046: mouse button release, including ready -> send failure ----

#[test]
fn t046_release_failure_stays_pending() {
    let mut d = settled_with(with_buttons(MouseButtons::LEFT));
    d.set_desired(&Desired::NEUTRAL);
    assert_eq!(send(&mut d, false), mouse(MouseButtons::EMPTY, 0, 0, 0));
    assert!(d.is_pending(ReportKind::Mouse));
    assert!(!d.is_settled());
    assert_eq!(drain_ok(&mut d), vec![mouse(MouseButtons::EMPTY, 0, 0, 0)]);
    assert!(d.is_settled());
}

// THE stuck-key trap: a "failed" press may still have reached the host. If the
// user then releases, delivered must not be assumed neutral -- send the release.
#[test]
fn t046_failed_press_then_release_still_sends_release() {
    let mut d = Delivery::new(true);
    d.set_desired(&with_buttons(MouseButtons::LEFT));
    send(&mut d, false); // host may or may not have LEFT down now
    d.set_desired(&Desired::NEUTRAL);
    assert!(d.is_pending(ReportKind::Mouse));
    assert_eq!(drain_ok(&mut d), vec![mouse(MouseButtons::EMPTY, 0, 0, 0)]);
}

#[test]
fn t046_same_trap_for_keyboard() {
    let mut d = Delivery::new(true);
    d.set_desired(&with_keys(&[Key::ENTER]));
    send(&mut d, false);
    d.set_desired(&Desired::NEUTRAL);
    assert_eq!(drain_ok(&mut d), vec![kb_out(&[])]);
}

// delivered changes only on success: a failed release leaves the old belief gone,
// so even returning to the old state resends it
#[test]
fn t046_after_failure_even_the_old_state_is_resent() {
    let mut d = settled_with(with_buttons(MouseButtons::LEFT));
    d.set_desired(&Desired::NEUTRAL);
    send(&mut d, false);
    d.set_desired(&with_buttons(MouseButtons::LEFT));
    assert_eq!(drain_ok(&mut d), vec![mouse(MouseButtons::LEFT, 0, 0, 0)]);
}

// ---- T047: consumer ----

#[test]
fn t047_consumer_release_is_retried_until_delivered() {
    let mut d = settled_with(with_consumer(ConsumerControls::VOL_UP));
    d.set_desired(&Desired::NEUTRAL);
    for _ in 0..3 {
        assert_eq!(send(&mut d, false), cc_out(ConsumerControls::EMPTY));
    }
    assert_eq!(drain_ok(&mut d), vec![cc_out(ConsumerControls::EMPTY)]);
    assert!(d.is_settled());
}

// held volume is one report, not one per frame (no repeated firing from us)
#[test]
fn t047_held_consumer_sent_once() {
    let mut d = Delivery::new(true);
    d.set_desired(&with_consumer(ConsumerControls::VOL_UP));
    assert_eq!(drain_ok(&mut d).len(), 1);
    for _ in 0..5 {
        d.set_desired(&with_consumer(ConsumerControls::VOL_UP));
        assert_eq!(d.next(), None);
    }
}

// Steam-mode descriptor has no 0x03: a delivery built without consumer never sends it
#[test]
fn without_consumer_never_sends_0x03() {
    let mut d = Delivery::new(false);
    d.set_desired(&with_consumer(ConsumerControls::VOL_UP));
    assert!(!d.is_pending(ReportKind::Consumer));
    assert_eq!(d.next(), None);
    d.release_all();
    d.on_bus_reset();
    assert_eq!(d.next(), None);
}

// ---- T048: one endpoint, several report kinds ----

#[test]
fn t048_round_robin_order() {
    let mut d = Delivery::new(true);
    let mut all = with_keys(&[Key::ENTER]);
    all.mouse.buttons = MouseButtons::LEFT;
    all.consumer.controls = ConsumerControls::VOL_UP;
    d.set_desired(&all);
    let kinds: Vec<ReportKind> = drain_ok(&mut d).iter().map(|r| r.kind()).collect();
    assert_eq!(kinds, vec![ReportKind::Mouse, ReportKind::Keyboard, ReportKind::Consumer]);
}

// a keyboard that keeps failing must not starve a pending mouse release
#[test]
fn t048_failing_keyboard_does_not_starve_mouse() {
    let mut start = with_keys(&[Key::ENTER]);
    start.mouse.buttons = MouseButtons::LEFT;
    let mut d = settled_with(start);
    d.set_desired(&Desired::NEUTRAL);

    let mut mouse_released = false;
    for _ in 0..4 {
        let r = d.next().unwrap();
        let ok = r.kind() == ReportKind::Mouse;
        d.on_sent(ok);
        mouse_released |= ok;
    }
    assert!(mouse_released);
    assert!(!d.is_pending(ReportKind::Mouse));
    assert!(d.is_pending(ReportKind::Keyboard));
}

// continuous cursor motion (a new mouse report every frame) must not starve keys
#[test]
fn t048_continuous_motion_does_not_starve_keyboard() {
    let mut d = Delivery::new(true);
    let mut frame = with_motion(3, 0, 0);
    frame.keyboard = kbd(Modifiers::EMPTY, &[Key::ENTER]);
    let mut kinds = Vec::new();
    for _ in 0..4 {
        d.set_desired(&frame);
        kinds.push(send(&mut d, true).kind());
    }
    assert!(kinds.contains(&ReportKind::Keyboard), "{kinds:?}");
}

// a successful keyboard send must not discard a pending mouse release
#[test]
fn t048_keyboard_success_keeps_mouse_pending() {
    let mut start = with_keys(&[Key::ENTER]);
    start.mouse.buttons = MouseButtons::LEFT;
    let mut d = settled_with(start);
    d.set_desired(&Desired::NEUTRAL);
    // mouse goes first and fails, keyboard goes next and succeeds
    assert_eq!(send(&mut d, false).kind(), ReportKind::Mouse);
    assert_eq!(send(&mut d, true).kind(), ReportKind::Keyboard);
    assert!(d.is_pending(ReportKind::Mouse));
    assert_eq!(drain_ok(&mut d), vec![mouse(MouseButtons::EMPTY, 0, 0, 0)]);
}

// ---- T049: release everything ----

fn everything_held() -> Desired {
    let mut d = Desired::NEUTRAL;
    d.keyboard = kbd(Modifiers::LCTRL, &[Key::ENTER]);
    d.mouse.buttons = MouseButtons::LEFT;
    d.consumer.controls = ConsumerControls::VOL_DOWN;
    d
}

#[test]
fn t049_release_all_drains_every_kind() {
    let mut d = settled_with(everything_held());
    d.release_all();
    assert!(!d.is_settled());
    let mut sent = drain_ok(&mut d);
    sent.sort_by_key(|r| r.kind());
    assert_eq!(
        sent,
        vec![
            mouse(MouseButtons::EMPTY, 0, 0, 0),
            kb_out(&[]),
            cc_out(ConsumerControls::EMPTY),
        ]
    );
    assert!(d.is_settled());
}

#[test]
fn t049_release_all_survives_failures() {
    let mut d = settled_with(everything_held());
    d.release_all();
    for _ in 0..6 {
        send(&mut d, false);
    }
    assert!(!d.is_settled());
    assert_eq!(drain_ok(&mut d).len(), 3);
    assert!(d.is_settled());
}

// motion from before the handoff is dropped, never flung after it
#[test]
fn t049_release_all_drops_pending_motion() {
    let mut d = Delivery::new(true);
    d.set_desired(&with_motion(50, 50, 5));
    d.release_all();
    assert_eq!(d.next(), None);
}

#[test]
fn t049_release_all_while_in_flight() {
    let mut d = Delivery::new(true);
    d.set_desired(&with_keys(&[Key::ENTER]));
    d.next();
    d.release_all();
    d.on_sent(true); // the press landed after all
    assert_eq!(drain_ok(&mut d), vec![kb_out(&[])]);
}

#[test]
fn t049_nothing_held_means_nothing_to_release() {
    let mut d = Delivery::new(true);
    d.release_all();
    assert!(d.is_settled());
}

// release is not a latch: the next desired state goes out normally
#[test]
fn t049_desired_after_release_is_sent() {
    let mut d = settled_with(everything_held());
    d.release_all();
    drain_ok(&mut d);
    d.set_desired(&with_keys(&[Key::TAB]));
    assert_eq!(drain_ok(&mut d), vec![kb_out(&[Key::TAB])]);
}

// ---- bus reset / replug (B8) ----

// re-enumeration wipes host state; a still-held key must be pressed again
#[test]
fn bus_reset_reasserts_held_state() {
    let mut d = settled_with(with_keys(&[Key::ENTER]));
    d.on_bus_reset();
    assert_eq!(drain_ok(&mut d), vec![kb_out(&[Key::ENTER])]);
}

#[test]
fn bus_reset_with_nothing_held_is_settled() {
    let mut d = settled_with(Desired::NEUTRAL);
    d.on_bus_reset();
    assert!(d.is_settled());
}

// a completion for a transfer from before the reset is ignored
#[test]
fn bus_reset_drops_the_in_flight_report() {
    let mut d = Delivery::new(true);
    d.set_desired(&with_keys(&[Key::ENTER]));
    d.next();
    d.on_bus_reset();
    assert_eq!(d.in_flight(), None);
    d.on_sent(true); // stale completion
    assert_eq!(drain_ok(&mut d), vec![kb_out(&[Key::ENTER])]);
}

#[test]
fn bus_reset_drops_pending_motion() {
    let mut d = Delivery::new(true);
    d.set_desired(&with_motion(40, 0, 0));
    d.on_bus_reset();
    assert_eq!(d.next(), None);
}

// a host that never stops failing: delivery keeps trying, never claims settled
#[test]
fn never_settles_while_sends_fail() {
    let mut d = Delivery::new(true);
    d.set_desired(&with_keys(&[Key::ENTER]));
    for _ in 0..100 {
        send(&mut d, false);
        assert!(!d.is_settled());
    }
}

// ---- property: whatever happens, the host ends up with the desired state ----

#[derive(Clone, Debug)]
enum Op {
    Set(Desired),
    Send { ok: bool, landed_anyway: bool },
    ReleaseAll,
    BusReset,
}

#[derive(Default, Debug, PartialEq)]
struct Host {
    buttons: MouseButtons,
    keyboard: KeyboardReport,
    consumer: ConsumerReport,
}

impl Host {
    fn apply(&mut self, r: &OutReport) {
        match r {
            OutReport::Mouse(m) => self.buttons = m.buttons,
            OutReport::Keyboard(k) => self.keyboard = *k,
            OutReport::Consumer(c) => self.consumer = *c,
        }
    }
}

fn arb_desired() -> impl Strategy<Value = Desired> {
    let keys = [Key::A, Key::O, Key::ENTER, Key::TAB];
    (0u8..=7, any::<u8>(), 0u8..16, 0u8..=3, -60i8..60).prop_map(move |(b, m, kmask, c, x)| {
        let mut d = Desired::NEUTRAL;
        d.mouse.buttons = MouseButtons::from_bits(b);
        d.mouse.x = x;
        d.keyboard.modifiers = Modifiers::from_bits(m);
        for (i, &k) in keys.iter().enumerate() {
            if kmask & (1 << i) != 0 {
                d.keyboard.press(k);
            }
        }
        d.consumer.controls = ConsumerControls::from_bits(c);
        d
    })
}

fn arb_op() -> impl Strategy<Value = Op> {
    prop_oneof![
        4 => arb_desired().prop_map(Op::Set),
        6 => (any::<bool>(), any::<bool>()).prop_map(|(ok, landed_anyway)| Op::Send { ok, landed_anyway }),
        1 => Just(Op::ReleaseAll),
        1 => Just(Op::BusReset),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1024))]

    // Failed sends may or may not reach the host (landed_anyway). After any history,
    // setting a final state and letting sends succeed leaves the host EXACTLY there.
    #[test]
    fn host_converges_to_desired(ops in prop::collection::vec(arb_op(), 0..60),
                                 last in arb_desired(),
                                 consumer in any::<bool>()) {
        let mut d = Delivery::new(consumer);
        let mut host = Host::default();
        for op in ops {
            match op {
                Op::Set(x) => d.set_desired(&x),
                Op::Send { ok, landed_anyway } => {
                    if let Some(r) = d.next() {
                        if ok || landed_anyway {
                            host.apply(&r);
                        }
                        d.on_sent(ok);
                    }
                }
                Op::ReleaseAll => d.release_all(),
                Op::BusReset => {
                    host = Host::default();
                    d.on_bus_reset();
                }
            }
        }
        d.set_desired(&last);
        for r in drain_ok(&mut d) {
            host.apply(&r);
        }
        prop_assert!(d.is_settled());
        prop_assert_eq!(host.buttons, last.mouse.buttons);
        prop_assert_eq!(host.keyboard, last.keyboard);
        let want_cc = if consumer { last.consumer } else { ConsumerReport::NEUTRAL };
        prop_assert_eq!(host.consumer, want_cc);
    }

    // With every send succeeding and the backlog under the clamp, motion is exact:
    // nothing lost, nothing invented. At most 6 frames of |x| < 20 pile up between sends.
    #[test]
    fn motion_is_conserved(chunks in prop::collection::vec(prop::collection::vec(-19i8..20, 1..=6), 1..30)) {
        let mut d = Delivery::new(true);
        let (mut asked, mut moved) = (0i32, 0i32);
        for chunk in chunks {
            for x in chunk {
                d.set_desired(&with_motion(x, 0, 0));
                asked += x as i32;
            }
            for r in drain_ok(&mut d) {
                if let OutReport::Mouse(m) = r {
                    moved += m.x as i32;
                }
            }
        }
        prop_assert_eq!(moved, asked);
    }
}
