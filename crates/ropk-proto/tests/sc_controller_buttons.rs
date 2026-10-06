// Contract for ropk_proto::sc_controller_buttons -- the controller's native button bitfield

use ropk_proto::sc_controller_buttons::{Button, Buttons};

// ---- Button: bit values ----

// A typo in one discriminant silently remaps a button in every mode, so pin
// each against the protocol table.
#[test]
fn button_bits_match_protocol_table() {
    let table: [(Button, u32); 30] = [
        (Button::A, 0x0000_0001),
        (Button::B, 0x0000_0002),
        (Button::X, 0x0000_0004),
        (Button::Y, 0x0000_0008),
        (Button::SteamQuickAccess, 0x0000_0010),
        (Button::RStick, 0x0000_0020),
        (Button::Select, 0x0000_0040),
        (Button::R4, 0x0000_0080),
        (Button::R5, 0x0000_0100),
        (Button::Rb, 0x0000_0200),
        (Button::DpadDown, 0x0000_0400),
        (Button::DpadRight, 0x0000_0800),
        (Button::DpadLeft, 0x0000_1000),
        (Button::DpadUp, 0x0000_2000),
        (Button::Start, 0x0000_4000),
        (Button::LStick, 0x0000_8000),
        (Button::Steam, 0x0001_0000),
        (Button::L4, 0x0002_0000),
        (Button::L5, 0x0004_0000),
        (Button::Lb, 0x0008_0000),
        (Button::RStickTouch, 0x0010_0000),
        (Button::RTrackpadTouch, 0x0020_0000),
        (Button::RTrackpadClick, 0x0040_0000),
        (Button::RT, 0x0080_0000),
        (Button::LStickTouch, 0x0100_0000),
        (Button::LTrackpadTouch, 0x0200_0000),
        (Button::LTrackpadClick, 0x0400_0000),
        (Button::LT, 0x0800_0000),
        (Button::RGripTouch, 0x1000_0000),
        (Button::LGripTouch, 0x2000_0000),
    ];
    for (b, bits) in table {
        assert_eq!(b.bit(), bits, "{b:?}");
    }
}

#[test]
fn all_list_is_complete_unique_and_in_bit_order() {
    let mut acc = 0u32;
    for (i, b) in Button::ALL.iter().enumerate() {
        assert_eq!(b.bit(), 1 << i, "ALL[{i}] = {b:?}");
        acc |= b.bit();
    }
    assert_eq!(acc, Buttons::PHYSICAL.bits());
}

#[test]
fn physical_covers_bits_0_to_29_only() {
    // bits 30/31 are never sent by the controller
    assert_eq!(Buttons::PHYSICAL.bits(), 0x3FFF_FFFF);
    assert_eq!(Buttons::PHYSICAL.bits().count_ones(), 30);
}

#[test]
fn group_constants() {
    assert_eq!(
        Buttons::BACK4,
        Button::L4 | Button::L5 | Button::R4 | Button::R5
    );
    assert_eq!(Buttons::BACK4.bits(), 0x0006_0180);
    assert_eq!(Buttons::GRIPS, Button::LGripTouch | Button::RGripTouch);
    assert_eq!(Buttons::GRIPS.bits(), 0x3000_0000);
}

// ---- Buttons: construction ----

#[test]
fn from_bits_roundtrip_keeps_every_bit() {
    // unknown/virtual bits are preserved: masking is the caller's choice
    for bits in [0, 1, 0x8000_0000, 0xFFFF_FFFF, 0x1234_5678] {
        assert_eq!(Buttons::from_bits(bits).bits(), bits);
    }
}

#[test]
fn default_is_empty() {
    assert_eq!(Buttons::default(), Buttons::EMPTY);
    assert!(Buttons::EMPTY.is_empty());
    assert!(!Buttons::from(Button::A).is_empty());
}

#[test]
fn from_button() {
    assert_eq!(Buttons::from(Button::Steam).bits(), 0x0001_0000);
}

#[test]
fn with_and_union_are_const_builders() {
    const AB: Buttons = Buttons::EMPTY.with(Button::A).with(Button::B);
    assert_eq!(AB.bits(), 0x3);
    assert_eq!(AB.union(Button::X.into()).bits(), 0x7);
}

// ---- Buttons: operators ----

#[test]
fn bitor_mixes_button_and_buttons() {
    let ab: Buttons = Button::A | Button::B;
    assert_eq!(ab.bits(), 0x3);
    assert_eq!((ab | Button::X).bits(), 0x7);
    assert_eq!((Button::Y | ab).bits(), 0xB);
    assert_eq!((ab | Buttons::GRIPS).bits(), 0x3000_0003);
}

#[test]
fn bitand_intersects() {
    let held = Button::A | Button::Steam;
    assert_eq!(held & Button::Steam, Buttons::from(Button::Steam));
    assert_eq!(held & Button::B, Buttons::EMPTY);
    assert_eq!(held & (Button::A | Button::B), Buttons::from(Button::A));
}

// ---- Buttons: queries ----

#[test]
fn has_checks_single_button() {
    let held = Button::A | Button::RTrackpadTouch;
    assert!(held.has(Button::A));
    assert!(held.has(Button::RTrackpadTouch));
    assert!(!held.has(Button::B));
}

#[test]
fn contains_requires_all_bits() {
    let three = Button::L4 | Button::L5 | Button::R4;
    assert!(!three.contains(Buttons::BACK4));
    assert!((three | Button::R5).contains(Buttons::BACK4));
    assert!(three.contains(Buttons::EMPTY));
}

#[test]
fn intersects_requires_any_bit() {
    let held = Buttons::from(Button::A);
    assert!(held.intersects(Button::A | Button::B));
    assert!(!held.intersects(Button::X | Button::Y));
    assert!(!held.intersects(Buttons::EMPTY));
}

// M5: lizardButtons() strips the grip bits so merely holding the controller
// can't trigger a binding (mode_lizard.cpp:78).
#[test]
fn remove_clears_only_given_bits() {
    let held = Button::A | Buttons::GRIPS;
    assert_eq!(held.remove(Buttons::GRIPS), Buttons::from(Button::A));
    assert_eq!(
        Buttons::from(Button::A).remove(Button::B.into()),
        Buttons::from(Button::A)
    );
    assert_eq!(Buttons::EMPTY.remove(Buttons::PHYSICAL), Buttons::EMPTY);
}

// ---- Buttons: iteration ----

#[test]
fn iter_yields_held_buttons_in_bit_order() {
    let held = Button::Lb | Button::A | Button::DpadUp;
    let mut it = held.iter();
    assert_eq!(it.next(), Some(Button::A));
    assert_eq!(it.next(), Some(Button::DpadUp));
    assert_eq!(it.next(), Some(Button::Lb));
    assert_eq!(it.next(), None);
}

#[test]
fn iter_skips_unknown_bits() {
    let held = Buttons::from_bits(0xC000_0001); // A + virtual bits 30/31
    let mut it = held.iter();
    assert_eq!(it.next(), Some(Button::A));
    assert_eq!(it.next(), None);
}

#[test]
fn iter_of_all_real_is_every_button() {
    assert_eq!(Buttons::PHYSICAL.iter().count(), 30);
    assert_eq!(Buttons::EMPTY.iter().count(), 0);
}
