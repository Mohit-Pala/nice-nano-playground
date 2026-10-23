// Contract for ropk_proto::lizard::eval -- walk the map against one controller's
// lizard word and OR its digital outputs into a shared accumulator.
// C: mode_lizard.cpp lizardEvalSlot() (called once per used slot by rfLizard()).
// Rules, in map order, per binding:
//   analog (MouseAxis / Scroll)  -> mark the source active, ignore masks
//   hold not all held            -> skip
//   trigger none held            -> skip
//   empty trigger                -> skip (BEHAVIOUR CHANGE, T042; C fired it)
//   MouseButton                  -> OR buttons
//   KbdChord                     -> skip if plain and its trigger was CLAIMED;
//                                   OR modifiers, press keys up to the first NONE;
//                                   a held chord CLAIMS its trigger bits
//   Consumer                     -> OR controls
// Claims are per eval() call (C: kbdConsumed is per controller), outputs accumulate.
// Covers T039 (behaviour of every default binding), T040, T042, part of T041.

use ropk_proto::hid::consumer::ConsumerControls;
use ropk_proto::hid::keyboard::{Key, KeyboardReport, Modifiers};
use ropk_proto::hid::mouse::MouseButtons;
use ropk_proto::lizard::buttons::{LizardButtons, StickDir};
use ropk_proto::lizard::eval::{LizardOutputs, eval};
use ropk_proto::lizard::map::{Binding, LizardMap, MouseSource, Output, ScrollSource};
use ropk_proto::sc_controller_buttons::Button;

const NONE: LizardButtons = LizardButtons::EMPTY;

fn btn(b: Button) -> LizardButtons {
    LizardButtons::button(b)
}

fn held(bs: &[Button]) -> LizardButtons {
    bs.iter().fold(NONE, |a, &b| a | btn(b))
}

fn run(map: &LizardMap, buttons: LizardButtons) -> LizardOutputs {
    let mut out = LizardOutputs::EMPTY;
    eval(map, buttons, &mut out);
    out
}

fn default_run(bs: &[Button]) -> LizardOutputs {
    run(&LizardMap::default_map(), held(bs))
}

fn kb(mods: Modifiers, keys: &[Key]) -> KeyboardReport {
    let mut r = KeyboardReport::NEUTRAL;
    r.modifiers = mods;
    for &k in keys {
        assert!(r.press(k));
    }
    r
}

fn keys_only(keys: &[Key]) -> KeyboardReport {
    kb(Modifiers::EMPTY, keys)
}

// digital part only: what the host would see from buttons
fn assert_digital(
    o: &LizardOutputs,
    keyboard: KeyboardReport,
    mouse: MouseButtons,
    consumer: ConsumerControls,
) {
    assert_eq!(o.keyboard, keyboard, "keyboard");
    assert_eq!(o.mouse_buttons, mouse, "mouse");
    assert_eq!(o.consumer, consumer, "consumer");
}

fn assert_only_keys(o: &LizardOutputs, keyboard: KeyboardReport) {
    assert_digital(o, keyboard, MouseButtons::EMPTY, ConsumerControls::EMPTY);
}

// ---- LizardOutputs ----

#[test]
fn empty_outputs() {
    let o = LizardOutputs::EMPTY;
    assert!(o.keyboard.is_neutral());
    assert_eq!(o.mouse_buttons, MouseButtons::EMPTY);
    assert_eq!(o.consumer, ConsumerControls::EMPTY);
    assert_eq!(o.mouse, None);
    assert_eq!(o.scroll, None);
    assert_eq!(LizardOutputs::default(), o);
}

// ---- analog sources ----

#[test]
fn default_map_enables_both_analog_sources_with_nothing_held() {
    let o = default_run(&[]);
    assert_eq!(o.mouse, Some(MouseSource::RightTrackpad));
    assert_eq!(o.scroll, Some(ScrollSource::LeftTrackpad));
    assert_only_keys(&o, KeyboardReport::NEUTRAL);
}

// C: MOUSE_AXIS / SCROLL `continue` before the mask checks
#[test]
fn analog_ignores_masks() {
    let map = LizardMap::from_slice(&[Binding::new(
        Output::MouseAxis(MouseSource::RightTrackpad),
        btn(Button::A),
        btn(Button::Steam),
    )])
    .unwrap();
    assert_eq!(run(&map, NONE).mouse, Some(MouseSource::RightTrackpad));
}

#[test]
fn empty_map_does_nothing() {
    let o = run(&LizardMap::EMPTY, LizardButtons::from_bits(u32::MAX));
    assert_eq!(o, LizardOutputs::EMPTY);
}

// ---- T039: every default binding on its own ----

#[test]
fn default_plain_keys() {
    let table = [
        (Button::A, Key::ENTER),
        (Button::B, Key::ESCAPE),
        (Button::X, Key::PAGE_UP),
        (Button::Y, Key::PAGE_DOWN),
        (Button::Select, Key::TAB),
        (Button::Start, Key::ESCAPE),
        (Button::DpadUp, Key::ARROW_UP),
        (Button::DpadDown, Key::ARROW_DOWN),
        (Button::DpadLeft, Key::ARROW_LEFT),
        (Button::DpadRight, Key::ARROW_RIGHT),
    ];
    for (b, k) in table {
        assert_only_keys(&default_run(&[b]), keys_only(&[k]));
    }
}

#[test]
fn default_stick_keys() {
    let table = [
        (StickDir::Up, Key::ARROW_UP),
        (StickDir::Down, Key::ARROW_DOWN),
        (StickDir::Left, Key::ARROW_LEFT),
        (StickDir::Right, Key::ARROW_RIGHT),
    ];
    for (d, k) in table {
        let o = run(&LizardMap::default_map(), LizardButtons::stick(d));
        assert_only_keys(&o, keys_only(&[k]));
    }
}

#[test]
fn default_mouse_buttons() {
    let table = [
        (Button::RTrackpadClick, MouseButtons::LEFT),
        (Button::RT, MouseButtons::LEFT),
        (Button::LT, MouseButtons::RIGHT),
        (Button::LTrackpadClick, MouseButtons::MIDDLE),
    ];
    for (b, m) in table {
        assert_digital(
            &default_run(&[b]),
            KeyboardReport::NEUTRAL,
            m,
            ConsumerControls::EMPTY,
        );
    }
}

#[test]
fn default_shoulders_are_modifier_only() {
    assert_only_keys(&default_run(&[Button::Lb]), kb(Modifiers::LCTRL, &[]));
    assert_only_keys(&default_run(&[Button::Rb]), kb(Modifiers::LALT, &[]));
}

#[test]
fn shoulders_combine_with_a_key() {
    let o = default_run(&[Button::Lb, Button::Rb, Button::A]);
    assert_only_keys(&o, kb(Modifiers::LCTRL | Modifiers::LALT, &[Key::ENTER]));
}

// buttons the default map doesn't bind do nothing
#[test]
fn unbound_buttons_do_nothing() {
    for b in [
        Button::LStick,
        Button::RStick,
        Button::SteamQuickAccess,
        Button::R4,
        Button::LStickTouch,
        Button::RStickTouch,
        Button::LTrackpadTouch,
        Button::RTrackpadTouch,
    ] {
        assert_only_keys(&default_run(&[b]), KeyboardReport::NEUTRAL);
    }
}

// ---- T042: Steam-held shortcuts and the claim rule ----

#[test]
fn steam_alone_does_nothing() {
    assert_only_keys(&default_run(&[Button::Steam]), KeyboardReport::NEUTRAL);
}

// the core rule: the chord claims X, so the plain X -> PageUp is suppressed
#[test]
fn steam_x_is_osk_not_page_up() {
    let o = default_run(&[Button::Steam, Button::X]);
    assert_only_keys(&o, kb(Modifiers::LGUI | Modifiers::LCTRL, &[Key::O]));
}

#[test]
fn x_alone_is_page_up() {
    assert_only_keys(&default_run(&[Button::X]), keys_only(&[Key::PAGE_UP]));
}

#[test]
fn steam_l4_is_ctrl_alt_delete() {
    let o = default_run(&[Button::Steam, Button::L4]);
    assert_only_keys(&o, kb(Modifiers::LCTRL | Modifiers::LALT, &[Key::DELETE]));
}

#[test]
fn l4_l5_r5_alone_do_nothing() {
    for b in [Button::L4, Button::L5, Button::R5] {
        assert_eq!(default_run(&[b]).keyboard, KeyboardReport::NEUTRAL, "{b:?}");
        assert_eq!(default_run(&[b]).consumer, ConsumerControls::EMPTY, "{b:?}");
    }
}

#[test]
fn steam_l5_r5_are_volume() {
    assert_digital(
        &default_run(&[Button::Steam, Button::L5]),
        KeyboardReport::NEUTRAL,
        MouseButtons::EMPTY,
        ConsumerControls::VOL_DOWN,
    );
    assert_digital(
        &default_run(&[Button::Steam, Button::R5]),
        KeyboardReport::NEUTRAL,
        MouseButtons::EMPTY,
        ConsumerControls::VOL_UP,
    );
}

// A has no Steam chord, so nothing claims it
#[test]
fn steam_does_not_block_unclaimed_keys() {
    assert_only_keys(
        &default_run(&[Button::Steam, Button::A]),
        keys_only(&[Key::ENTER]),
    );
}

// C quirk kept: claims only suppress KEYBOARD bindings; mouse and consumer still fire
#[test]
fn claims_do_not_suppress_mouse_buttons() {
    let o = default_run(&[Button::Steam, Button::RT]);
    assert_eq!(o.mouse_buttons, MouseButtons::LEFT);
}

// a consumer binding with a hold doesn't claim anything
#[test]
fn consumer_does_not_claim() {
    let map = LizardMap::from_slice(&[
        Binding::new(Output::Consumer(ConsumerControls::VOL_UP), btn(Button::A), btn(Button::Steam)),
        Binding::new(Output::chord(Modifiers::EMPTY, Key::ENTER), btn(Button::A), NONE),
    ])
    .unwrap();
    let o = run(&map, held(&[Button::Steam, Button::A]));
    assert_digital(&o, keys_only(&[Key::ENTER]), MouseButtons::EMPTY, ConsumerControls::VOL_UP);
}

// Order matters: with the plain binding FIRST, nothing is claimed yet, so both fire.
// This is why DEFAULT_MAP keeps held chords first (lizard_map test checks that).
#[test]
fn claim_only_affects_later_bindings() {
    let map = LizardMap::from_slice(&[
        Binding::new(Output::chord(Modifiers::EMPTY, Key::PAGE_UP), btn(Button::X), NONE),
        Binding::new(
            Output::chord(Modifiers::LGUI, Key::O),
            btn(Button::X),
            btn(Button::Steam),
        ),
    ])
    .unwrap();
    let o = run(&map, held(&[Button::Steam, Button::X]));
    assert_only_keys(&o, kb(Modifiers::LGUI, &[Key::PAGE_UP, Key::O]));
}

// a claim only happens when the held chord actually fires
#[test]
fn unfired_chord_claims_nothing() {
    let o = default_run(&[Button::X]);
    assert_only_keys(&o, keys_only(&[Key::PAGE_UP]));
}

// a claimed trigger with several bits suppresses a plain binding that shares ANY of them
#[test]
fn claim_matches_any_shared_bit() {
    let map = LizardMap::from_slice(&[
        Binding::new(
            Output::chord(Modifiers::LCTRL, Key::NONE),
            btn(Button::X) | btn(Button::Y),
            btn(Button::Steam),
        ),
        Binding::new(Output::chord(Modifiers::EMPTY, Key::PAGE_DOWN), btn(Button::Y), NONE),
    ])
    .unwrap();
    let o = run(&map, held(&[Button::Steam, Button::X, Button::Y]));
    assert_only_keys(&o, kb(Modifiers::LCTRL, &[]));
}

// ---- masks ----

#[test]
fn trigger_is_any_of() {
    let map = LizardMap::from_slice(&[Binding::new(
        Output::chord(Modifiers::EMPTY, Key::A),
        btn(Button::RT) | btn(Button::LT),
        NONE,
    )])
    .unwrap();
    assert_eq!(run(&map, btn(Button::RT)).keyboard, keys_only(&[Key::A]));
    assert_eq!(run(&map, btn(Button::LT)).keyboard, keys_only(&[Key::A]));
    assert_eq!(run(&map, btn(Button::A)).keyboard, KeyboardReport::NEUTRAL);
}

#[test]
fn hold_is_all_of() {
    let map = LizardMap::from_slice(&[Binding::new(
        Output::chord(Modifiers::EMPTY, Key::A),
        btn(Button::A),
        btn(Button::Steam) | btn(Button::Lb),
    )])
    .unwrap();
    assert_eq!(
        run(&map, held(&[Button::A, Button::Steam])).keyboard,
        KeyboardReport::NEUTRAL
    );
    assert_eq!(
        run(&map, held(&[Button::A, Button::Steam, Button::Lb])).keyboard,
        keys_only(&[Key::A])
    );
}

// BEHAVIOUR CHANGE (T042): C fired a digital binding whose trigger was empty --
// always-on with no hold, or "hold-only" with one. Here it never fires.
#[test]
fn empty_trigger_never_fires() {
    let map = LizardMap::from_slice(&[
        Binding::new(Output::chord(Modifiers::EMPTY, Key::A), NONE, NONE),
        Binding::new(Output::chord(Modifiers::EMPTY, Key::O), NONE, btn(Button::Steam)),
        Binding::new(Output::MouseButton(MouseButtons::LEFT), NONE, NONE),
        Binding::new(Output::Consumer(ConsumerControls::VOL_UP), NONE, NONE),
    ])
    .unwrap();
    for buttons in [NONE, btn(Button::Steam), LizardButtons::from_bits(u32::MAX)] {
        assert_eq!(run(&map, buttons), LizardOutputs::EMPTY, "{buttons:?}");
    }
}

// ---- chords ----

// C: `for (j = 1; j < 7 && b.outData[j]; j++)` -- keys stop at the first NONE
#[test]
fn chord_keys_stop_at_first_none() {
    let map = LizardMap::from_slice(&[Binding::new(
        Output::KbdChord {
            modifiers: Modifiers::EMPTY,
            keys: [Key::A, Key::NONE, Key::O, Key::NONE, Key::NONE, Key::NONE],
        },
        btn(Button::A),
        NONE,
    )])
    .unwrap();
    assert_eq!(run(&map, btn(Button::A)).keyboard, keys_only(&[Key::A]));
}

#[test]
fn chord_with_several_keys() {
    let map = LizardMap::from_slice(&[Binding::new(
        Output::KbdChord {
            modifiers: Modifiers::LSHIFT,
            keys: [Key::A, Key::O, Key::TAB, Key::NONE, Key::NONE, Key::NONE],
        },
        btn(Button::A),
        NONE,
    )])
    .unwrap();
    assert_eq!(
        run(&map, btn(Button::A)).keyboard,
        kb(Modifiers::LSHIFT, &[Key::A, Key::O, Key::TAB])
    );
}

// ---- T040: duplicates and overflow (D2) ----

#[test]
fn dpad_and_stick_up_use_one_slot() {
    let o = run(
        &LizardMap::default_map(),
        btn(Button::DpadUp) | LizardButtons::stick(StickDir::Up),
    );
    assert_only_keys(&o, keys_only(&[Key::ARROW_UP]));
}

#[test]
fn b_and_start_both_escape_once() {
    assert_only_keys(
        &default_run(&[Button::B, Button::Start]),
        keys_only(&[Key::ESCAPE]),
    );
}

#[test]
fn two_bindings_same_mouse_button() {
    let o = default_run(&[Button::RT, Button::RTrackpadClick]);
    assert_eq!(o.mouse_buttons, MouseButtons::LEFT);
}

// seven distinct keys: the first six IN MAP ORDER win, the seventh is dropped
#[test]
fn seventh_distinct_key_dropped_in_map_order() {
    let o = default_run(&[
        Button::A,
        Button::B,
        Button::X,
        Button::Y,
        Button::Select,
        Button::DpadUp,
        Button::DpadDown,
    ]);
    assert_only_keys(
        &o,
        keys_only(&[
            Key::ENTER,
            Key::ESCAPE,
            Key::PAGE_UP,
            Key::PAGE_DOWN,
            Key::TAB,
            Key::ARROW_UP,
        ]),
    );
}

// modifiers still apply when the key slots are full
#[test]
fn modifiers_survive_key_overflow() {
    let o = default_run(&[
        Button::Lb,
        Button::A,
        Button::B,
        Button::X,
        Button::Y,
        Button::Select,
        Button::DpadUp,
        Button::DpadDown,
    ]);
    assert_eq!(o.keyboard.modifiers, Modifiers::LCTRL);
    assert!(o.keyboard.is_full());
}

// ---- accumulating across controllers (rfLizard calls eval once per slot) ----

#[test]
fn outputs_accumulate_across_calls() {
    let map = LizardMap::default_map();
    let mut out = LizardOutputs::EMPTY;
    eval(&map, btn(Button::A), &mut out);
    eval(&map, btn(Button::LT), &mut out);
    assert_digital(&out, keys_only(&[Key::ENTER]), MouseButtons::RIGHT, ConsumerControls::EMPTY);
}

// T041 groundwork: both controllers hold A -> one Enter
#[test]
fn same_key_from_two_controllers_is_one_key() {
    let map = LizardMap::default_map();
    let mut out = LizardOutputs::EMPTY;
    eval(&map, btn(Button::A), &mut out);
    eval(&map, btn(Button::A), &mut out);
    assert_eq!(out.keyboard, keys_only(&[Key::ENTER]));
}

// claims don't leak between controllers: one holds Steam+X, the other just X
#[test]
fn claims_are_per_call() {
    let map = LizardMap::default_map();
    let mut out = LizardOutputs::EMPTY;
    eval(&map, held(&[Button::Steam, Button::X]), &mut out);
    eval(&map, btn(Button::X), &mut out);
    assert_eq!(
        out.keyboard,
        kb(Modifiers::LGUI | Modifiers::LCTRL, &[Key::O, Key::PAGE_UP])
    );
}

// eval never clears what's already there
#[test]
fn eval_only_adds() {
    let map = LizardMap::default_map();
    let mut out = LizardOutputs::EMPTY;
    eval(&map, held(&[Button::Lb, Button::A]), &mut out);
    let before = out;
    eval(&map, NONE, &mut out);
    assert_eq!(out, before);
}
