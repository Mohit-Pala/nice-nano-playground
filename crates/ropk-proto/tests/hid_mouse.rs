// Contract for ropk_proto::hid::mouse -- the lizard desktop mouse, report 0x40.
// Rides on slot 0's HID interface (interface 2), same descriptor as the slots.
// C: puck_hid.cpp PUCK_HID_DESC / PUCK_LIZARD_HID_DESC (report 0x40),
//    mode_lizard.cpp rfLizard() "mouse report" + rfLizardRelease(),
//    TinyUSB hid_mouse_report_t {buttons, x, y, wheel, pan}.
// Wire: [0x40][buttons][x][y][wheel][pan], axes are i8 relative, logical -127..127.
// Groundwork for T043 (descriptor/report agreement is checked in A4).

use proptest::prelude::*;
use ropk_proto::hid::mouse::{
    AXIS_MAX, AXIS_MIN, MOUSE_BODY_LEN, MOUSE_REPORT_LEN, MouseButtons, MouseReport,
    REPORT_ID_MOUSE, clamp_axis,
};

fn report(buttons: MouseButtons, x: i8, y: i8, wheel: i8, pan: i8) -> MouseReport {
    MouseReport { buttons, x, y, wheel, pan }
}

// ---- constants ----

#[test]
fn report_id_is_0x40() {
    assert_eq!(REPORT_ID_MOUSE, 0x40);
}

#[test]
fn lengths_are_body_5_plus_id() {
    assert_eq!(MOUSE_BODY_LEN, 5);
    assert_eq!(MOUSE_REPORT_LEN, 6);
    assert_eq!(MOUSE_REPORT_LEN, MOUSE_BODY_LEN + 1);
}

// descriptor: Logical Minimum 0x81 (-127), Logical Maximum 0x7F (127)
#[test]
fn axis_range_matches_descriptor() {
    assert_eq!(AXIS_MIN, -127);
    assert_eq!(AXIS_MAX, 127);
}

// ---- buttons ----

// HID button page: usage 1 = bit 0 = left, 2 = right, 3 = middle.
// The C map uses these exact bits (lizard_map.cpp addMouseBtn(1|2|4, ..)).
#[test]
fn button_bits_match_hid_and_c_map() {
    assert_eq!(MouseButtons::LEFT.bits(), 0x01);
    assert_eq!(MouseButtons::RIGHT.bits(), 0x02);
    assert_eq!(MouseButtons::MIDDLE.bits(), 0x04);
    assert_eq!(MouseButtons::EMPTY.bits(), 0x00);
    assert_eq!(MouseButtons::ALL.bits(), 0x07);
}

#[test]
fn from_bits_masks_padding_bits() {
    // bits 3..7 are descriptor padding (Input Const); they must never reach the wire
    assert_eq!(MouseButtons::from_bits(0xFF), MouseButtons::ALL);
    assert_eq!(MouseButtons::from_bits(0xF8), MouseButtons::EMPTY);
    assert_eq!(MouseButtons::from_bits(0x09), MouseButtons::LEFT);
}

#[test]
fn set_operations() {
    let lr = MouseButtons::LEFT | MouseButtons::RIGHT;
    assert_eq!(lr.bits(), 0x03);
    assert!(lr.contains(MouseButtons::LEFT));
    assert!(lr.contains(MouseButtons::RIGHT));
    assert!(!lr.contains(MouseButtons::MIDDLE));
    assert!(lr.contains(MouseButtons::EMPTY));

    assert_eq!(lr.with(MouseButtons::MIDDLE), MouseButtons::ALL);
    assert_eq!(lr.remove(MouseButtons::LEFT), MouseButtons::RIGHT);
    assert_eq!(lr.union(MouseButtons::MIDDLE), MouseButtons::ALL);

    assert!(MouseButtons::EMPTY.is_empty());
    assert!(!lr.is_empty());
}

// rfLizard ORs every binding's button into one byte; doing it twice is harmless
#[test]
fn or_is_idempotent() {
    let b = MouseButtons::LEFT | MouseButtons::LEFT;
    assert_eq!(b, MouseButtons::LEFT);
}

#[test]
fn default_buttons_are_empty() {
    assert_eq!(MouseButtons::default(), MouseButtons::EMPTY);
}

// ---- clamp_axis ----

#[test]
fn clamp_axis_passes_in_range_values() {
    for v in [-127, -1, 0, 1, 5, 127] {
        assert_eq!(clamp_axis(v) as i32, v, "v={v}");
    }
}

// C: rfLizard clamps dx/dy to +-127 before the (int8_t) cast
#[test]
fn clamp_axis_saturates() {
    assert_eq!(clamp_axis(128), 127);
    assert_eq!(clamp_axis(10_000), 127);
    assert_eq!(clamp_axis(i32::MAX), 127);
    assert_eq!(clamp_axis(-128), -127);
    assert_eq!(clamp_axis(-10_000), -127);
    assert_eq!(clamp_axis(i32::MIN), -127);
}

// ---- neutral ----

#[test]
fn neutral_is_all_zero() {
    let n = MouseReport::NEUTRAL;
    assert_eq!(n, report(MouseButtons::EMPTY, 0, 0, 0, 0));
    assert!(n.is_neutral());
    assert!(!n.has_motion());
    assert_eq!(MouseReport::default(), n);
}

// C rfLizardRelease sends {0,0,0,0,0}
#[test]
fn neutral_encodes_to_id_and_zeros() {
    assert_eq!(MouseReport::NEUTRAL.encode(), [0x40, 0, 0, 0, 0, 0]);
}

#[test]
fn button_held_is_not_neutral_and_not_motion() {
    let r = report(MouseButtons::LEFT, 0, 0, 0, 0);
    assert!(!r.is_neutral());
    assert!(!r.has_motion());
}

#[test]
fn any_axis_is_motion() {
    for r in [
        report(MouseButtons::EMPTY, 1, 0, 0, 0),
        report(MouseButtons::EMPTY, 0, -1, 0, 0),
        report(MouseButtons::EMPTY, 0, 0, 1, 0),
        report(MouseButtons::EMPTY, 0, 0, 0, -1),
    ] {
        assert!(r.has_motion(), "{r:?}");
        assert!(!r.is_neutral(), "{r:?}");
    }
}

// ---- encode ----

#[test]
fn encode_layout() {
    let r = report(MouseButtons::RIGHT, 5, -3, 2, -1);
    assert_eq!(r.encode(), [0x40, 0x02, 0x05, 0xFD, 0x02, 0xFF]);
}

#[test]
fn encode_extremes() {
    let r = report(MouseButtons::ALL, 127, -127, 127, -127);
    assert_eq!(r.encode(), [0x40, 0x07, 0x7F, 0x81, 0x7F, 0x81]);
}

// i8::MIN is outside the descriptor's logical range; send -127 instead
#[test]
fn encode_never_emits_minus_128() {
    let r = report(MouseButtons::EMPTY, -128, -128, -128, -128);
    assert_eq!(r.encode(), [0x40, 0x00, 0x81, 0x81, 0x81, 0x81]);
}

#[test]
fn encode_is_always_6_bytes_starting_with_id() {
    let e = report(MouseButtons::MIDDLE, 1, 2, 3, 4).encode();
    assert_eq!(e.len(), MOUSE_REPORT_LEN);
    assert_eq!(e[0], REPORT_ID_MOUSE);
}

// ---- decode ----

#[test]
fn decode_layout() {
    let r = MouseReport::decode(&[0x40, 0x02, 0x05, 0xFD, 0x02, 0xFF]).unwrap();
    assert_eq!(r, report(MouseButtons::RIGHT, 5, -3, 2, -1));
}

#[test]
fn decode_rejects_wrong_id() {
    assert_eq!(MouseReport::decode(&[0x41, 0, 0, 0, 0, 0]), None);
    assert_eq!(MouseReport::decode(&[0x00, 0, 0, 0, 0, 0]), None);
}

#[test]
fn decode_rejects_every_short_length() {
    let full = [0x40, 0x01, 0x01, 0x01, 0x01, 0x01];
    for n in 0..MOUSE_REPORT_LEN {
        assert_eq!(MouseReport::decode(&full[..n]), None, "len={n}");
    }
}

// trailing bytes (e.g. a padded 64-byte endpoint buffer) are ignored
#[test]
fn decode_ignores_trailing_bytes() {
    let mut buf = [0xAAu8; 64];
    buf[..6].copy_from_slice(&[0x40, 0x01, 0x0A, 0x00, 0x00, 0x00]);
    assert_eq!(
        MouseReport::decode(&buf),
        Some(report(MouseButtons::LEFT, 10, 0, 0, 0))
    );
}

#[test]
fn decode_masks_padding_bits() {
    let r = MouseReport::decode(&[0x40, 0xF9, 0, 0, 0, 0]).unwrap();
    assert_eq!(r.buttons, MouseButtons::LEFT);
}

// ---- properties ----

fn arb_axis() -> impl Strategy<Value = i8> {
    AXIS_MIN..=AXIS_MAX
}

proptest! {
    // every in-range report survives encode -> decode unchanged
    #[test]
    fn roundtrip(b in 0u8..=7, x in arb_axis(), y in arb_axis(), w in arb_axis(), p in arb_axis()) {
        let r = report(MouseButtons::from_bits(b), x, y, w, p);
        prop_assert_eq!(MouseReport::decode(&r.encode()), Some(r));
    }

    // whatever goes in, the wire stays inside the descriptor's contract
    #[test]
    fn wire_is_always_valid(b in any::<u8>(), x in any::<i8>(), y in any::<i8>(), w in any::<i8>(), p in any::<i8>()) {
        let e = report(MouseButtons::from_bits(b), x, y, w, p).encode();
        prop_assert_eq!(e[0], REPORT_ID_MOUSE);
        prop_assert_eq!(e[1] & !0x07, 0);
        for &axis in &e[2..] {
            prop_assert_ne!(axis, 0x80);
        }
    }

    #[test]
    fn clamp_axis_stays_in_range(v in any::<i32>()) {
        let c = clamp_axis(v);
        prop_assert!((AXIS_MIN..=AXIS_MAX).contains(&c));
    }
}
