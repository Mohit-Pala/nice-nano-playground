// Contract for ropk_proto::lizard::map -- the lizard binding table and its default.
// A binding = (trigger any-of, hold all-of, one output). Eval (A7) walks the table
// in order every frame; ORDER MATTERS (a Steam+X chord must come before plain X).
// C: lizard_map.h LizardBinding / LizardMap / LZ_OUT_* / LZ_MAX_BINDINGS,
//    lizard_map.cpp defaultLizardMap().
// Covers T039 (every default binding), T056 count bounds, groundwork for T042/T044/T057.

use ropk_proto::hid::consumer::ConsumerControls;
use ropk_proto::hid::keyboard::{Key, Modifiers};
use ropk_proto::hid::mouse::MouseButtons;
use ropk_proto::lizard::buttons::{LizardButtons, StickDir};
use ropk_proto::lizard::map::{
    Binding, DEFAULT_MAP, LizardMap, MAX_BINDINGS, MapFull, MouseSource, Output, ScrollSource,
};
use ropk_proto::sc_controller_buttons::Button;

const NONE: LizardButtons = LizardButtons::EMPTY;

const fn btn(b: Button) -> LizardButtons {
    LizardButtons::button(b)
}

const fn stick(d: StickDir) -> LizardButtons {
    LizardButtons::stick(d)
}

fn key(k: Key, trig: LizardButtons) -> Binding {
    Binding::new(Output::chord(Modifiers::EMPTY, k), trig, NONE)
}

// lizard_map.cpp defaultLizardMap(), entry for entry, in C order
fn c_default() -> Vec<Binding> {
    let steam = btn(Button::Steam);
    vec![
        // analog sources
        Binding::new(Output::MouseAxis(MouseSource::RightTrackpad), NONE, NONE),
        Binding::new(Output::Scroll(ScrollSource::LeftTrackpad), NONE, NONE),
        // mouse buttons
        Binding::new(Output::MouseButton(MouseButtons::LEFT), btn(Button::RTrackpadClick), NONE),
        Binding::new(Output::MouseButton(MouseButtons::LEFT), btn(Button::RT), NONE),
        Binding::new(Output::MouseButton(MouseButtons::RIGHT), btn(Button::LT), NONE),
        Binding::new(Output::MouseButton(MouseButtons::MIDDLE), btn(Button::LTrackpadClick), NONE),
        // Steam-held shortcuts (before the plain X / L4 bindings)
        Binding::new(
            Output::chord(Modifiers::LCTRL | Modifiers::LALT, Key::DELETE),
            btn(Button::L4),
            steam,
        ),
        Binding::new(
            Output::chord(Modifiers::LGUI | Modifiers::LCTRL, Key::O),
            btn(Button::X),
            steam,
        ),
        Binding::new(Output::Consumer(ConsumerControls::VOL_DOWN), btn(Button::L5), steam),
        Binding::new(Output::Consumer(ConsumerControls::VOL_UP), btn(Button::R5), steam),
        // keys
        key(Key::ENTER, btn(Button::A)),
        key(Key::ESCAPE, btn(Button::B)),
        key(Key::PAGE_UP, btn(Button::X)),
        key(Key::PAGE_DOWN, btn(Button::Y)),
        key(Key::TAB, btn(Button::Select)),
        key(Key::ESCAPE, btn(Button::Start)),
        // d-pad
        key(Key::ARROW_UP, btn(Button::DpadUp)),
        key(Key::ARROW_DOWN, btn(Button::DpadDown)),
        key(Key::ARROW_LEFT, btn(Button::DpadLeft)),
        key(Key::ARROW_RIGHT, btn(Button::DpadRight)),
        // left stick (virtual bits)
        key(Key::ARROW_UP, stick(StickDir::Up)),
        key(Key::ARROW_DOWN, stick(StickDir::Down)),
        key(Key::ARROW_LEFT, stick(StickDir::Left)),
        key(Key::ARROW_RIGHT, stick(StickDir::Right)),
        // shoulders: modifier-only chords
        Binding::new(Output::chord(Modifiers::LCTRL, Key::NONE), btn(Button::Lb), NONE),
        Binding::new(Output::chord(Modifiers::LALT, Key::NONE), btn(Button::Rb), NONE),
    ]
}

fn is_analog(o: &Output) -> bool {
    matches!(o, Output::MouseAxis(_) | Output::Scroll(_))
}

// ---- constants ----

#[test]
fn max_bindings_is_32() {
    assert_eq!(MAX_BINDINGS, 32);
}

// ---- Output::chord ----

#[test]
fn chord_puts_the_key_first_and_pads_with_none() {
    let o = Output::chord(Modifiers::LCTRL, Key::DELETE);
    assert_eq!(
        o,
        Output::KbdChord {
            modifiers: Modifiers::LCTRL,
            keys: [Key::DELETE, Key::NONE, Key::NONE, Key::NONE, Key::NONE, Key::NONE],
        }
    );
}

#[test]
fn chord_with_none_is_modifier_only() {
    let o = Output::chord(Modifiers::LALT, Key::NONE);
    assert_eq!(
        o,
        Output::KbdChord { modifiers: Modifiers::LALT, keys: [Key::NONE; 6] }
    );
}

// ---- Binding ----

#[test]
fn binding_new_stores_fields() {
    let b = Binding::new(
        Output::MouseButton(MouseButtons::LEFT),
        btn(Button::RT),
        btn(Button::Steam),
    );
    assert_eq!(b.output, Output::MouseButton(MouseButtons::LEFT));
    assert_eq!(b.trigger, btn(Button::RT));
    assert_eq!(b.hold, btn(Button::Steam));
}

// DEFAULT_MAP is a const, so these must be const-callable
#[test]
fn constructors_are_const() {
    const B: Binding = Binding::new(Output::chord(Modifiers::EMPTY, Key::A), LizardButtons::button(Button::A), LizardButtons::EMPTY);
    assert_eq!(B.trigger.bits(), 0x1);
}

// ---- DEFAULT_MAP: T039 ----

#[test]
fn default_map_has_26_entries() {
    assert_eq!(DEFAULT_MAP.len(), 26);
}

#[test]
fn default_map_matches_c_entry_by_entry() {
    let expect = c_default();
    assert_eq!(DEFAULT_MAP.len(), expect.len());
    for (i, (got, want)) in DEFAULT_MAP.iter().zip(&expect).enumerate() {
        assert_eq!(got, want, "entry {i}");
    }
}

// analog sources are always on: C's MOUSE_AXIS/SCROLL ignore trig/hold
#[test]
fn analog_bindings_have_no_masks() {
    for (i, b) in DEFAULT_MAP.iter().enumerate() {
        if is_analog(&b.output) {
            assert!(b.trigger.is_empty() && b.hold.is_empty(), "entry {i}");
        }
    }
}

// T042: a digital output with no trigger would be always-on (or hold-only) -- never in the default
#[test]
fn every_digital_binding_has_a_trigger() {
    for (i, b) in DEFAULT_MAP.iter().enumerate() {
        if !is_analog(&b.output) {
            assert!(!b.trigger.is_empty(), "entry {i}: {b:?}");
        }
    }
}

// C's kbdConsumed only works if a held chord comes BEFORE every plain binding
// that shares its trigger (Steam+X before X -> PageUp).
#[test]
fn held_chords_precede_plain_bindings_on_the_same_trigger() {
    for (i, held) in DEFAULT_MAP.iter().enumerate() {
        if held.hold.is_empty() {
            continue;
        }
        for (j, plain) in DEFAULT_MAP.iter().enumerate() {
            if plain.hold.is_empty() && plain.trigger.intersects(held.trigger) {
                assert!(i < j, "held entry {i} must come before plain entry {j}");
            }
        }
    }
}

// grips can't be represented (they alias the stick bits); nothing else may sneak in either
#[test]
fn every_mask_uses_known_bits_only() {
    let mut known = 0u32;
    for b in Button::ALL {
        if b != Button::RGripTouch && b != Button::LGripTouch {
            known |= b.bit();
        }
    }
    for d in StickDir::ALL {
        known |= d.bit();
    }
    for (i, b) in DEFAULT_MAP.iter().enumerate() {
        assert_eq!(b.trigger.bits() & !known, 0, "entry {i} trigger");
        assert_eq!(b.hold.bits() & !known, 0, "entry {i} hold");
    }
}

// the default only uses the sources C actually implements (T044)
#[test]
fn default_uses_implemented_sources_only() {
    let mouse = DEFAULT_MAP
        .iter()
        .filter(|b| b.output == Output::MouseAxis(MouseSource::RightTrackpad))
        .count();
    let scroll = DEFAULT_MAP
        .iter()
        .filter(|b| b.output == Output::Scroll(ScrollSource::LeftTrackpad))
        .count();
    assert_eq!((mouse, scroll), (1, 1));
}

// ---- LizardMap ----

#[test]
fn empty_map_is_empty() {
    let m = LizardMap::EMPTY;
    assert!(m.is_empty());
    assert_eq!(m.len(), 0);
    assert!(m.bindings().is_empty());
    assert!(!m.is_full());
}

#[test]
fn default_map_container_holds_default_map() {
    let m = LizardMap::default_map();
    assert_eq!(m.len(), DEFAULT_MAP.len());
    assert_eq!(m.bindings(), &DEFAULT_MAP[..]);
}

// T057 groundwork: "empty" and "defaults" are different maps
#[test]
fn empty_is_not_default() {
    assert_ne!(LizardMap::EMPTY, LizardMap::default_map());
}

#[test]
fn push_appends_in_order() {
    let mut m = LizardMap::EMPTY;
    let a = key(Key::ENTER, btn(Button::A));
    let b = key(Key::ESCAPE, btn(Button::B));
    assert_eq!(m.push(a), Ok(()));
    assert_eq!(m.push(b), Ok(()));
    assert_eq!(m.bindings(), &[a, b]);
}

// T056: 32 fit, the 33rd is refused and the map is unchanged
#[test]
fn push_accepts_32_and_refuses_33() {
    let mut m = LizardMap::EMPTY;
    let b = key(Key::A, btn(Button::A));
    for i in 0..MAX_BINDINGS {
        assert_eq!(m.push(b), Ok(()), "push {i}");
    }
    assert!(m.is_full());
    let before = m;
    assert_eq!(m.push(key(Key::O, btn(Button::B))), Err(MapFull));
    assert_eq!(m, before);
    assert_eq!(m.len(), MAX_BINDINGS);
}

#[test]
fn from_slice_sizes() {
    let b = key(Key::A, btn(Button::A));
    for n in [0, 1, 26, 32] {
        let v = vec![b; n];
        let m = LizardMap::from_slice(&v).unwrap();
        assert_eq!(m.len(), n, "n={n}");
    }
    assert_eq!(LizardMap::from_slice(&vec![b; 33]), Err(MapFull));
}

#[test]
fn from_slice_keeps_order() {
    let m = LizardMap::from_slice(&DEFAULT_MAP).unwrap();
    assert_eq!(m, LizardMap::default_map());
}

