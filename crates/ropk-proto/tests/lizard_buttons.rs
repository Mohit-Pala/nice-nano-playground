// Contract for ropk_proto::lizard::buttons -- the 32-bit word the lizard map matches on.
// bits 0..27  = the controller's physical buttons, untouched
// bits 28..31 = VIRTUAL left-stick directions (|axis| past 12000)
// The controller uses bits 28/29 for grip touch; C clears the top 4 bits first so
// just holding the controller can't look like a stick push (grips aren't bindable).
// C: mode_lizard.cpp lizardButtons(), lizard_map.h LZ_BTN_LSTICK_*, triton.h TB_*.
// Groundwork for T039/T042 (A6/A7 match bindings against this word).

use proptest::prelude::*;
use ropk_proto::input::ControllerInput;
use ropk_proto::lizard::buttons::{
    LizardButtons, PHYSICAL_MASK, STICK_THRESHOLD, StickDir, VIRTUAL_MASK,
};
use ropk_proto::sc_controller_buttons::{Button, Buttons};

fn input(buttons: u32, lx: i16, ly: i16) -> ControllerInput {
    let mut i = ControllerInput::NEUTRAL;
    i.buttons = Buttons::from_bits(buttons);
    i.sticks.lx = lx;
    i.sticks.ly = ly;
    i
}

fn word(buttons: u32, lx: i16, ly: i16) -> u32 {
    LizardButtons::from_input(&input(buttons, lx, ly)).bits()
}

// ---- constants ----

#[test]
fn masks_split_the_word() {
    assert_eq!(PHYSICAL_MASK, 0x0FFF_FFFF);
    assert_eq!(VIRTUAL_MASK, 0xF000_0000);
    assert_eq!(PHYSICAL_MASK & VIRTUAL_MASK, 0);
    assert_eq!(PHYSICAL_MASK | VIRTUAL_MASK, u32::MAX);
}

#[test]
fn threshold_is_12000() {
    assert_eq!(STICK_THRESHOLD, 12_000);
}

// lizard_map.h LZ_BTN_LSTICK_RT/LF/DN/UP
#[test]
fn stick_bits_match_c() {
    assert_eq!(StickDir::Right.bit(), 0x1000_0000);
    assert_eq!(StickDir::Left.bit(), 0x2000_0000);
    assert_eq!(StickDir::Down.bit(), 0x4000_0000);
    assert_eq!(StickDir::Up.bit(), 0x8000_0000);
    assert_eq!(StickDir::ALL.len(), 4);
    let all = StickDir::ALL.iter().fold(0, |a, d| a | d.bit());
    assert_eq!(all, VIRTUAL_MASK);
}

// Pins every Button the C default map (lizard_map.cpp) binds to the TB_* value
// it was written against. If a Button bit ever moves, the map silently breaks.
#[test]
fn map_buttons_match_c_tb_values() {
    let table: [(Button, u32, &str); 22] = [
        (Button::A, 0x1, "TB_A"),
        (Button::B, 0x2, "TB_B"),
        (Button::X, 0x4, "TB_X"),
        (Button::Y, 0x8, "TB_Y"),
        (Button::Select, 0x40, "TB_VIEW"),
        (Button::R5, 0x100, "TB_R5"),
        (Button::Rb, 0x200, "TB_RB"),
        (Button::DpadDown, 0x400, "TB_DDN"),
        (Button::DpadRight, 0x800, "TB_DRT"),
        (Button::DpadLeft, 0x1000, "TB_DLF"),
        (Button::DpadUp, 0x2000, "TB_DUP"),
        (Button::Start, 0x4000, "TB_MENU"),
        (Button::Steam, 0x10000, "TB_STEAM"),
        (Button::L4, 0x20000, "TB_L4"),
        (Button::L5, 0x40000, "TB_L5"),
        (Button::Lb, 0x80000, "TB_LB"),
        (Button::RTrackpadTouch, 0x200000, "TB_RPADT"),
        (Button::RTrackpadClick, 0x400000, "TB_RPADC"),
        (Button::RT, 0x800000, "TB_R2"),
        (Button::LTrackpadTouch, 0x2000000, "TB_LPADT"),
        (Button::LTrackpadClick, 0x4000000, "TB_LPADC"),
        (Button::LT, 0x8000000, "TB_L2"),
    ];
    for (b, c, name) in table {
        assert_eq!(b.bit(), c, "{b:?} vs {name}");
    }
}

// the grips are exactly the physical bits that collide with the virtual ones
#[test]
fn grips_live_in_the_virtual_range() {
    assert_eq!(Button::RGripTouch.bit() & VIRTUAL_MASK, Button::RGripTouch.bit());
    assert_eq!(Button::LGripTouch.bit() & VIRTUAL_MASK, Button::LGripTouch.bit());
    for b in Button::ALL {
        if b != Button::RGripTouch && b != Button::LGripTouch {
            assert_eq!(b.bit() & VIRTUAL_MASK, 0, "{b:?}");
        }
    }
}

// ---- from_input: physical part ----

#[test]
fn neutral_input_is_empty() {
    let w = LizardButtons::from_input(&ControllerInput::NEUTRAL);
    assert_eq!(w, LizardButtons::EMPTY);
    assert!(w.is_empty());
}

#[test]
fn every_non_grip_button_passes_through() {
    for b in Button::ALL {
        if b == Button::RGripTouch || b == Button::LGripTouch {
            continue;
        }
        assert_eq!(word(b.bit(), 0, 0), b.bit(), "{b:?}");
    }
}

// The bug the C comment describes: holding the controller sets the grip bits,
// which equal LSTICK_RT / LSTICK_LF. They must not fire a stick binding.
#[test]
fn grips_never_become_stick_bits() {
    let grips = Buttons::GRIPS.bits();
    assert_eq!(word(grips, 0, 0), 0);
    assert_eq!(word(grips | Button::A.bit(), 0, 0), Button::A.bit());
}

// bits 30/31 are unassigned on the wire today; whatever arrives there is dropped too
#[test]
fn raw_top_bits_are_cleared() {
    assert_eq!(word(0xF000_0000, 0, 0), 0);
    assert_eq!(word(0xFFFF_FFFF, 0, 0), PHYSICAL_MASK);
}

// ---- from_input: virtual stick part ----

#[test]
fn threshold_is_strict() {
    let t = STICK_THRESHOLD;
    assert_eq!(word(0, t, 0), 0);
    assert_eq!(word(0, -t, 0), 0);
    assert_eq!(word(0, 0, t), 0);
    assert_eq!(word(0, 0, -t), 0);

    assert_eq!(word(0, t + 1, 0), StickDir::Right.bit());
    assert_eq!(word(0, -t - 1, 0), StickDir::Left.bit());
    assert_eq!(word(0, 0, t + 1), StickDir::Up.bit());
    assert_eq!(word(0, 0, -t - 1), StickDir::Down.bit());
}

// C: ly > 12000 = UP, ly < -12000 = DOWN (stick Y is positive-up)
#[test]
fn y_positive_is_up() {
    assert_eq!(word(0, 0, i16::MAX), StickDir::Up.bit());
    assert_eq!(word(0, 0, i16::MIN), StickDir::Down.bit());
}

#[test]
fn extremes() {
    assert_eq!(word(0, i16::MAX, 0), StickDir::Right.bit());
    assert_eq!(word(0, i16::MIN, 0), StickDir::Left.bit());
}

#[test]
fn diagonal_sets_two_directions() {
    let w = word(0, 20_000, 20_000);
    assert_eq!(w, StickDir::Right.bit() | StickDir::Up.bit());
    let w = word(0, -20_000, -20_000);
    assert_eq!(w, StickDir::Left.bit() | StickDir::Down.bit());
}

// only the LEFT stick feeds the virtual bits
#[test]
fn right_stick_is_ignored() {
    let mut i = ControllerInput::NEUTRAL;
    i.sticks.rx = i16::MAX;
    i.sticks.ry = i16::MIN;
    assert_eq!(LizardButtons::from_input(&i), LizardButtons::EMPTY);
}

#[test]
fn buttons_and_stick_combine() {
    let w = word(Button::Lb.bit() | Buttons::GRIPS.bits(), 0, 13_000);
    assert_eq!(w, Button::Lb.bit() | StickDir::Up.bit());
}

// ---- constructors and set operations (what A6 masks are built from) ----

#[test]
fn button_constructor() {
    assert_eq!(LizardButtons::button(Button::X).bits(), 0x4);
    assert_eq!(LizardButtons::button(Button::LT).bits(), 0x800_0000);
}

#[test]
fn stick_constructor() {
    assert_eq!(LizardButtons::stick(StickDir::Up).bits(), 0x8000_0000);
}

// building a mask in a const is how DEFAULT_MAP (A6) will use these
#[test]
fn constructors_are_const() {
    const STEAM_X: LizardButtons =
        LizardButtons::button(Button::Steam).union(LizardButtons::button(Button::X));
    const UP: LizardButtons = LizardButtons::stick(StickDir::Up);
    assert_eq!(STEAM_X.bits(), 0x10004);
    assert_eq!(UP.bits(), 0x8000_0000);
}

// a grip "button" in the lizard word would alias LSTICK_RT/LF: refuse it
#[test]
#[should_panic]
fn button_constructor_rejects_right_grip() {
    let _ = LizardButtons::button(Button::RGripTouch);
}

#[test]
#[should_panic]
fn button_constructor_rejects_left_grip() {
    let _ = LizardButtons::button(Button::LGripTouch);
}

#[test]
fn from_bits_is_raw() {
    for b in [0, 1, 0x1000_0000, 0xFFFF_FFFF] {
        assert_eq!(LizardButtons::from_bits(b).bits(), b);
    }
}

// holdMask: ALL bits held (lizard_map.h "AND-guard")
#[test]
fn contains_is_all_of() {
    let held = LizardButtons::button(Button::Steam) | LizardButtons::button(Button::X);
    assert!(held.contains(LizardButtons::button(Button::Steam)));
    assert!(held.contains(held));
    assert!(held.contains(LizardButtons::EMPTY));
    assert!(!LizardButtons::button(Button::X).contains(held));
}

// trigMask: ANY bit held (lizard_map.h "any-of (OR)")
#[test]
fn intersects_is_any_of() {
    let rt_or_rpadc = LizardButtons::button(Button::RT) | LizardButtons::button(Button::RTrackpadClick);
    assert!(LizardButtons::button(Button::RT).intersects(rt_or_rpadc));
    assert!(LizardButtons::button(Button::RTrackpadClick).intersects(rt_or_rpadc));
    assert!(!LizardButtons::button(Button::A).intersects(rt_or_rpadc));
    assert!(!rt_or_rpadc.intersects(LizardButtons::EMPTY));
}

#[test]
fn default_is_empty() {
    assert_eq!(LizardButtons::default(), LizardButtons::EMPTY);
}

// ---- properties ----

proptest! {
    #[test]
    fn physical_bits_pass_through(b in any::<u32>(), lx in any::<i16>(), ly in any::<i16>()) {
        prop_assert_eq!(word(b, lx, ly) & PHYSICAL_MASK, b & PHYSICAL_MASK);
    }

    #[test]
    fn virtual_bits_follow_the_stick_only(b in any::<u32>(), lx in any::<i16>(), ly in any::<i16>()) {
        let w = word(b, lx, ly);
        let t = STICK_THRESHOLD;
        prop_assert_eq!(w & StickDir::Right.bit() != 0, lx > t);
        prop_assert_eq!(w & StickDir::Left.bit() != 0, lx < -t);
        prop_assert_eq!(w & StickDir::Up.bit() != 0, ly > t);
        prop_assert_eq!(w & StickDir::Down.bit() != 0, ly < -t);
    }

    #[test]
    fn opposite_directions_never_together(b in any::<u32>(), lx in any::<i16>(), ly in any::<i16>()) {
        let w = word(b, lx, ly);
        let lr = StickDir::Left.bit() | StickDir::Right.bit();
        let ud = StickDir::Up.bit() | StickDir::Down.bit();
        prop_assert_ne!(w & lr, lr);
        prop_assert_ne!(w & ud, ud);
    }
}
