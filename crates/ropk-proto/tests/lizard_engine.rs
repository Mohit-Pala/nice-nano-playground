// Contract for ropk_proto::lizard::engine -- one lizard frame, start to finish:
// per-slot samples -> drop stale -> lizard word -> eval -> pad motion -> the
// DESIRED mouse / keyboard / consumer reports. Delivery (A10) is separate.
// C: mode_lizard.cpp rfLizard() (merges every used slot onto one mouse+keyboard)
//    and rfLizardRelease() (state reset). Staleness: input.rs LINK_TIMEOUT_MS
//    (rf_link.cpp 300 ms).
// Covers T041 (shared keys across controllers), neutral after timeout/disconnect.

use ropk_proto::bond::NUM_SLOTS;
use ropk_proto::hid::consumer::{ConsumerControls, ConsumerReport};
use ropk_proto::hid::keyboard::{Key, KeyboardReport, Modifiers};
use ropk_proto::hid::mouse::{MouseButtons, MouseReport};
use ropk_proto::input::{ControllerInput, LINK_TIMEOUT_MS, Sample};
use ropk_proto::lizard::engine::{Desired, LizardEngine};
use ropk_proto::lizard::map::LizardMap;
use ropk_proto::lizard::motion::MotionConfig;
use ropk_proto::sc_controller_buttons::{Button, Buttons};

const NOW: u64 = 10_000;
const NO_SLOTS: [Option<Sample>; NUM_SLOTS] = [None; NUM_SLOTS];

fn engine() -> LizardEngine {
    LizardEngine::new(LizardMap::default_map(), MotionConfig::DEFAULT)
}

fn pressed(bs: &[Button]) -> ControllerInput {
    let mut i = ControllerInput::NEUTRAL;
    i.buttons = bs.iter().fold(Buttons::EMPTY, |a, &b| a.with(b));
    i
}

fn fresh(input: ControllerInput) -> Option<Sample> {
    Some(Sample { input, received_ms: NOW })
}

fn aged(input: ControllerInput, age_ms: u64) -> Option<Sample> {
    Some(Sample { input, received_ms: NOW - age_ms })
}

fn slot0(s: Option<Sample>) -> [Option<Sample>; NUM_SLOTS] {
    [s, None, None, None]
}

fn keys(ks: &[Key]) -> KeyboardReport {
    let mut r = KeyboardReport::NEUTRAL;
    for &k in ks {
        r.press(k);
    }
    r
}

fn rpad(touch: bool, x: i16, y: i16) -> ControllerInput {
    let mut i = ControllerInput::NEUTRAL;
    if touch {
        i.buttons = Buttons::EMPTY.with(Button::RTrackpadTouch);
    }
    i.r_trackpad.x = x;
    i.r_trackpad.y = y;
    i
}

fn lpad(touch: bool, y: i16) -> ControllerInput {
    let mut i = ControllerInput::NEUTRAL;
    if touch {
        i.buttons = Buttons::EMPTY.with(Button::LTrackpadTouch);
    }
    i.l_trackpad.y = y;
    i
}

// ---- Desired ----

#[test]
fn desired_neutral() {
    let n = Desired::NEUTRAL;
    assert_eq!(n.mouse, MouseReport::NEUTRAL);
    assert_eq!(n.keyboard, KeyboardReport::NEUTRAL);
    assert_eq!(n.consumer, ConsumerReport::NEUTRAL);
    assert!(n.is_neutral());
    assert_eq!(Desired::default(), n);
}

#[test]
fn desired_not_neutral_if_any_part_is_set() {
    let mut d = Desired::NEUTRAL;
    d.mouse.x = 1;
    assert!(!d.is_neutral());
    let mut d = Desired::NEUTRAL;
    d.keyboard.modifiers = Modifiers::LCTRL;
    assert!(!d.is_neutral());
    let mut d = Desired::NEUTRAL;
    d.consumer.controls = ConsumerControls::VOL_UP;
    assert!(!d.is_neutral());
}

// ---- construction ----

#[test]
fn new_keeps_map_and_config() {
    let cfg = MotionConfig::new(32, 50);
    let e = LizardEngine::new(LizardMap::EMPTY, cfg);
    assert_eq!(e.map, LizardMap::EMPTY);
    assert_eq!(e.config, cfg);
}

// the firmware keeps the engine in a static
#[test]
fn new_is_const() {
    const E: LizardEngine = LizardEngine::new(LizardMap::default_map(), MotionConfig::DEFAULT);
    assert_eq!(E.map.len(), 26);
}

// ---- basic frames ----

#[test]
fn nothing_connected_is_neutral() {
    assert_eq!(engine().step(&NO_SLOTS, NOW), Desired::NEUTRAL);
}

#[test]
fn connected_but_idle_is_neutral() {
    let d = engine().step(&slot0(fresh(ControllerInput::NEUTRAL)), NOW);
    assert_eq!(d, Desired::NEUTRAL);
}

#[test]
fn a_is_enter() {
    let d = engine().step(&slot0(fresh(pressed(&[Button::A]))), NOW);
    assert_eq!(d.keyboard, keys(&[Key::ENTER]));
    assert_eq!(d.mouse, MouseReport::NEUTRAL);
    assert_eq!(d.consumer, ConsumerReport::NEUTRAL);
}

#[test]
fn mouse_buttons_reach_the_mouse_report() {
    let d = engine().step(&slot0(fresh(pressed(&[Button::LT]))), NOW);
    assert_eq!(d.mouse.buttons, MouseButtons::RIGHT);
    assert!(d.keyboard.is_neutral());
}

#[test]
fn consumer_reaches_the_consumer_report() {
    let d = engine().step(&slot0(fresh(pressed(&[Button::Steam, Button::R5]))), NOW);
    assert_eq!(d.consumer.controls, ConsumerControls::VOL_UP);
}

#[test]
fn steam_x_end_to_end() {
    let d = engine().step(&slot0(fresh(pressed(&[Button::Steam, Button::X]))), NOW);
    let mut want = keys(&[Key::O]);
    want.modifiers = Modifiers::LGUI | Modifiers::LCTRL;
    assert_eq!(d.keyboard, want);
}

// A5 is wired in: stick past the threshold is an arrow; grips alone are nothing
#[test]
fn stick_and_grips_end_to_end() {
    let mut i = pressed(&[Button::LGripTouch, Button::RGripTouch]);
    assert!(engine().step(&slot0(fresh(i)), NOW).is_neutral());
    i.sticks.lx = 20_000;
    assert_eq!(engine().step(&slot0(fresh(i)), NOW).keyboard, keys(&[Key::ARROW_RIGHT]));
}

#[test]
fn pan_is_always_zero() {
    let mut e = engine();
    e.step(&slot0(fresh(rpad(true, 0, 0))), NOW);
    let d = e.step(&slot0(fresh(rpad(true, 20_000, 20_000))), NOW);
    assert_eq!(d.mouse.pan, 0);
}

// ---- staleness: neutral after timeout ----

#[test]
fn stale_sample_is_ignored() {
    let d = engine().step(&slot0(aged(pressed(&[Button::A]), LINK_TIMEOUT_MS)), NOW);
    assert_eq!(d, Desired::NEUTRAL);
}

#[test]
fn freshness_boundary() {
    let a = pressed(&[Button::A]);
    let d = engine().step(&slot0(aged(a, LINK_TIMEOUT_MS - 1)), NOW);
    assert_eq!(d.keyboard, keys(&[Key::ENTER]));
    let d = engine().step(&slot0(aged(a, LINK_TIMEOUT_MS)), NOW);
    assert!(d.is_neutral());
}

// a sample stamped slightly after `now` (task race) counts as fresh, not ancient
#[test]
fn future_timestamp_is_fresh() {
    let s = Some(Sample { input: pressed(&[Button::A]), received_ms: NOW + 5 });
    assert_eq!(engine().step(&slot0(s), NOW).keyboard, keys(&[Key::ENTER]));
}

// holding a key, then the link goes quiet: the key is released by the timeout
#[test]
fn held_key_released_when_link_goes_stale() {
    let mut e = engine();
    let a = pressed(&[Button::A]);
    let held = Some(Sample { input: a, received_ms: NOW });
    assert_eq!(e.step(&slot0(held), NOW).keyboard, keys(&[Key::ENTER]));
    assert_eq!(e.step(&slot0(held), NOW + 200).keyboard, keys(&[Key::ENTER]));
    assert!(e.step(&slot0(held), NOW + LINK_TIMEOUT_MS).is_neutral());
}

// ---- several controllers (T041) ----

#[test]
fn two_controllers_merge_in_slot_order() {
    let d = engine().step(
        &[fresh(pressed(&[Button::B])), fresh(pressed(&[Button::A])), None, None],
        NOW,
    );
    assert_eq!(d.keyboard.keys(), &[Key::ESCAPE, Key::ENTER]);
}

#[test]
fn any_slot_works() {
    for s in 0..NUM_SLOTS {
        let mut slots = NO_SLOTS;
        slots[s] = fresh(pressed(&[Button::A]));
        assert_eq!(engine().step(&slots, NOW).keyboard, keys(&[Key::ENTER]), "slot {s}");
    }
}

// T041: the key stays down until the LAST controller holding it lets go
#[test]
fn shared_key_held_until_last_owner_releases() {
    let mut e = engine();
    let a = fresh(pressed(&[Button::A]));
    let idle = fresh(ControllerInput::NEUTRAL);
    assert_eq!(e.step(&[a, a, None, None], NOW).keyboard, keys(&[Key::ENTER]));
    assert_eq!(e.step(&[idle, a, None, None], NOW).keyboard, keys(&[Key::ENTER]));
    assert_eq!(e.step(&[a, idle, None, None], NOW).keyboard, keys(&[Key::ENTER]));
    assert!(e.step(&[idle, idle, None, None], NOW).keyboard.is_neutral());
}

// T041 with link loss: one owner goes stale, the other still holds
#[test]
fn shared_key_survives_one_owner_going_stale() {
    let a_old = aged(pressed(&[Button::A]), LINK_TIMEOUT_MS);
    let a_new = fresh(pressed(&[Button::A]));
    let d = engine().step(&[a_old, a_new, None, None], NOW);
    assert_eq!(d.keyboard, keys(&[Key::ENTER]));
}

#[test]
fn shared_modifier_from_two_controllers() {
    let d = engine().step(
        &[fresh(pressed(&[Button::Lb])), fresh(pressed(&[Button::Rb, Button::A])), None, None],
        NOW,
    );
    let mut want = keys(&[Key::ENTER]);
    want.modifiers = Modifiers::LCTRL | Modifiers::LALT;
    assert_eq!(d.keyboard, want);
}

// ---- motion is wired in ----

#[test]
fn right_pad_moves_the_mouse() {
    let mut e = engine();
    e.config = MotionConfig::new(64, 0);
    assert_eq!(e.step(&slot0(fresh(rpad(true, 0, 0))), NOW).mouse.x, 0);
    let d = e.step(&slot0(fresh(rpad(true, 640, 640))), NOW);
    assert_eq!((d.mouse.x, d.mouse.y), (1, -1));
}

// pad click while dragging: buttons and motion in the same report
#[test]
fn click_and_move_together() {
    let mut e = engine();
    e.config = MotionConfig::new(64, 0);
    e.step(&slot0(fresh(rpad(true, 0, 0))), NOW);
    let mut i = rpad(true, 1280, 0);
    i.buttons = i.buttons.with(Button::RTrackpadClick);
    let d = e.step(&slot0(fresh(i)), NOW);
    assert_eq!((d.mouse.buttons, d.mouse.x), (MouseButtons::LEFT, 2));
}

#[test]
fn left_pad_scrolls() {
    let mut e = engine();
    e.step(&slot0(fresh(lpad(true, 0))), NOW);
    assert_eq!(e.step(&slot0(fresh(lpad(true, 1536))), NOW).mouse.wheel, 1);
}

#[test]
fn two_pads_one_cursor() {
    let mut e = engine();
    e.config = MotionConfig::new(64, 0);
    let a = fresh(rpad(true, 0, 0));
    e.step(&[a, a, None, None], NOW);
    let b = fresh(rpad(true, 320, 0));
    assert_eq!(e.step(&[b, b, None, None], NOW).mouse.x, 1);
}

// a controller that goes stale mid-flick stops gliding at once
#[test]
fn stale_slot_stops_glide() {
    let mut e = engine();
    e.step(&slot0(fresh(rpad(true, 0, 0))), NOW);
    let flick = e.step(&slot0(fresh(rpad(true, 20_000, 0))), NOW);
    assert!(flick.mouse.x > 0);
    let after = e.step(&slot0(aged(rpad(false, 20_000, 0), LINK_TIMEOUT_MS)), NOW);
    assert_eq!(after, Desired::NEUTRAL);
}

// no MouseAxis / Scroll binding -> no motion at all (C skips the whole block)
#[test]
fn map_without_analog_never_moves() {
    let mut e = LizardEngine::new(LizardMap::EMPTY, MotionConfig::new(64, 0));
    e.step(&slot0(fresh(rpad(true, 0, 0))), NOW);
    let d = e.step(&slot0(fresh(rpad(true, 20_000, 0))), NOW);
    assert_eq!(d, Desired::NEUTRAL);
    let mut l = lpad(true, 0);
    e.step(&slot0(fresh(l)), NOW);
    l.l_trackpad.y = 15_000;
    assert_eq!(e.step(&slot0(fresh(l)), NOW), Desired::NEUTRAL);
}

#[test]
fn config_is_read_every_step() {
    let mut e = engine();
    e.config = MotionConfig::new(64, 0);
    e.step(&slot0(fresh(rpad(true, 0, 0))), NOW);
    e.config = MotionConfig::new(32, 0); // twice as sensitive
    assert_eq!(e.step(&slot0(fresh(rpad(true, 640, 0))), NOW).mouse.x, 2);
}

#[test]
fn map_is_read_every_step() {
    let mut e = engine();
    let a = slot0(fresh(pressed(&[Button::A])));
    assert_eq!(e.step(&a, NOW).keyboard, keys(&[Key::ENTER]));
    e.map = LizardMap::EMPTY;
    assert!(e.step(&a, NOW).is_neutral());
}

// ---- reset (rfLizardRelease) ----

#[test]
fn reset_clears_motion_state() {
    let mut e = engine();
    e.step(&slot0(fresh(rpad(true, 0, 0))), NOW);
    e.step(&slot0(fresh(rpad(true, 20_000, 0))), NOW);
    e.reset();
    // next touch is a first touch: no jump, and the old glide is gone
    assert_eq!(e.step(&slot0(fresh(rpad(true, 20_000, 0))), NOW), Desired::NEUTRAL);
}

#[test]
fn reset_keeps_map_and_config() {
    let cfg = MotionConfig::new(32, 50);
    let mut e = LizardEngine::new(LizardMap::EMPTY, cfg);
    e.reset();
    assert_eq!(e.map, LizardMap::EMPTY);
    assert_eq!(e.config, cfg);
}

// digital outputs are recomputed every frame -- reset doesn't need to "release" them
#[test]
fn held_buttons_come_back_after_reset() {
    let mut e = engine();
    let a = slot0(fresh(pressed(&[Button::A])));
    e.step(&a, NOW);
    e.reset();
    assert_eq!(e.step(&a, NOW).keyboard, keys(&[Key::ENTER]));
}
