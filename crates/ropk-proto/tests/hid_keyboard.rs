// Contract for ropk_proto::hid::keyboard -- the lizard desktop keyboard, report 0x41.
// Rides on slot 0's HID interface (interface 2), same descriptor as the slots.
// C: puck_hid.cpp PUCK_HID_DESC (report 0x41), lizard_map.cpp KM_* + HID_KEY_*,
//    mode_lizard.cpp lizardEvalSlot() KBD_CHORD + rfLizard() "keyboard report".
// Wire: [0x41][modifiers][reserved 0][k0..k5], keys are usages 0x00..=0x65.
// Covers the report half of T040 (duplicates share one slot, overflow policy D2).

use proptest::prelude::*;
use ropk_proto::hid::keyboard::{
    KEYBOARD_BODY_LEN, KEYBOARD_REPORT_LEN, Key, KeyboardReport, MAX_KEYS, Modifiers,
    REPORT_ID_KEYBOARD,
};

fn report_with(mods: Modifiers, keys: &[Key]) -> KeyboardReport {
    let mut r = KeyboardReport::NEUTRAL;
    r.modifiers = mods;
    for &k in keys {
        assert!(r.press(k), "setup press {k:?}");
    }
    r
}

// six distinct valid keys
const SIX: [Key; 6] = [
    Key::ENTER,
    Key::ESCAPE,
    Key::TAB,
    Key::PAGE_UP,
    Key::PAGE_DOWN,
    Key::DELETE,
];

// ---- constants ----

#[test]
fn report_id_is_0x41() {
    assert_eq!(REPORT_ID_KEYBOARD, 0x41);
}

#[test]
fn lengths_are_body_8_plus_id() {
    assert_eq!(KEYBOARD_BODY_LEN, 8);
    assert_eq!(KEYBOARD_REPORT_LEN, 9);
    assert_eq!(KEYBOARD_REPORT_LEN, KEYBOARD_BODY_LEN + 1);
    assert_eq!(MAX_KEYS, 6);
}

// descriptor: Usage Minimum 0x00, Usage Maximum 0x65, Logical Maximum 0x65
#[test]
fn key_usage_range_matches_descriptor() {
    assert_eq!(Key::USAGE_MIN, 0x04);
    assert_eq!(Key::USAGE_MAX, 0x65);
}

// ---- modifiers ----

// HID usages 0xE0..0xE7 -> bits 0..7. C: lizard_map.cpp KM_LCTRL 0x01, KM_LSHIFT 0x02,
// KM_LALT 0x04, KM_LGUI 0x08.
#[test]
fn modifier_bits_match_hid_and_c() {
    assert_eq!(Modifiers::EMPTY.bits(), 0x00);
    assert_eq!(Modifiers::LCTRL.bits(), 0x01);
    assert_eq!(Modifiers::LSHIFT.bits(), 0x02);
    assert_eq!(Modifiers::LALT.bits(), 0x04);
    assert_eq!(Modifiers::LGUI.bits(), 0x08);
    assert_eq!(Modifiers::RCTRL.bits(), 0x10);
    assert_eq!(Modifiers::RSHIFT.bits(), 0x20);
    assert_eq!(Modifiers::RALT.bits(), 0x40);
    assert_eq!(Modifiers::RGUI.bits(), 0x80);
}

// all 8 bits are real modifiers, nothing to mask
#[test]
fn modifiers_from_bits_keeps_every_bit() {
    for b in [0x00, 0x05, 0x80, 0xFF] {
        assert_eq!(Modifiers::from_bits(b).bits(), b);
    }
}

#[test]
fn modifier_set_operations() {
    let ca = Modifiers::LCTRL | Modifiers::LALT;
    assert_eq!(ca.bits(), 0x05);
    assert!(ca.contains(Modifiers::LCTRL));
    assert!(!ca.contains(Modifiers::LGUI));
    assert!(ca.contains(Modifiers::EMPTY));
    assert_eq!(ca.with(Modifiers::LGUI).bits(), 0x0D);
    assert_eq!(ca.union(Modifiers::LGUI).bits(), 0x0D);
    assert_eq!(ca.remove(Modifiers::LCTRL), Modifiers::LALT);
    assert!(Modifiers::EMPTY.is_empty());
    assert!(!ca.is_empty());
    assert_eq!(Modifiers::default(), Modifiers::EMPTY);
}

// ---- keys ----

// TinyUSB HID_KEY_* values used by the C default map (lizard_map.cpp defaultLizardMap)
#[test]
fn key_usages_match_tinyusb() {
    assert_eq!(Key::NONE.usage(), 0x00);
    assert_eq!(Key::A.usage(), 0x04);
    assert_eq!(Key::O.usage(), 0x12);
    assert_eq!(Key::ENTER.usage(), 0x28);
    assert_eq!(Key::ESCAPE.usage(), 0x29);
    assert_eq!(Key::TAB.usage(), 0x2B);
    assert_eq!(Key::PAGE_UP.usage(), 0x4B);
    assert_eq!(Key::DELETE.usage(), 0x4C);
    assert_eq!(Key::PAGE_DOWN.usage(), 0x4E);
    assert_eq!(Key::ARROW_RIGHT.usage(), 0x4F);
    assert_eq!(Key::ARROW_LEFT.usage(), 0x50);
    assert_eq!(Key::ARROW_DOWN.usage(), 0x51);
    assert_eq!(Key::ARROW_UP.usage(), 0x52);
}

#[test]
fn from_usage_accepts_the_descriptor_range() {
    for u in Key::USAGE_MIN..=Key::USAGE_MAX {
        assert_eq!(Key::from_usage(u).map(Key::usage), Some(u), "usage={u:#04x}");
    }
}

// 0x00 = no key, 0x01..0x03 = keyboard error codes (ErrorRollOver, POSTFail,
// ErrorUndefined), > 0x65 is past the descriptor's Logical Maximum, 0xE0..0xE7
// are modifiers and travel in the modifier byte instead. None of these is a
// pressable key. Groundwork for T056 "invalid output codes".
#[test]
fn from_usage_rejects_everything_else() {
    for u in [0x00, 0x01, 0x02, 0x03, 0x66, 0x68, 0xE0, 0xE7, 0xFF] {
        assert_eq!(Key::from_usage(u), None, "usage={u:#04x}");
    }
}

#[test]
fn none_is_none() {
    assert!(Key::NONE.is_none());
    assert!(!Key::ENTER.is_none());
    assert_eq!(Key::default(), Key::NONE);
}

// ---- neutral ----

#[test]
fn neutral_is_empty() {
    let n = KeyboardReport::NEUTRAL;
    assert!(n.is_neutral());
    assert_eq!(n.modifiers, Modifiers::EMPTY);
    assert_eq!(n.len(), 0);
    assert!(n.keys().is_empty());
    assert!(!n.is_full());
    assert_eq!(KeyboardReport::default(), n);
}

// C rfLizardRelease sends krep[8] = {0}
#[test]
fn neutral_encodes_to_id_and_zeros() {
    assert_eq!(KeyboardReport::NEUTRAL.encode(), [0x41, 0, 0, 0, 0, 0, 0, 0, 0]);
}

// shoulder buttons in the C map are modifier-only chords (LB -> Ctrl, RB -> Alt)
#[test]
fn modifier_only_is_not_neutral() {
    let r = report_with(Modifiers::LCTRL, &[]);
    assert!(!r.is_neutral());
    assert_eq!(r.len(), 0);
}

#[test]
fn key_only_is_not_neutral() {
    let r = report_with(Modifiers::EMPTY, &[Key::ENTER]);
    assert!(!r.is_neutral());
}

// ---- press ----

#[test]
fn press_adds_in_order() {
    let mut r = KeyboardReport::NEUTRAL;
    assert!(r.press(Key::ARROW_UP));
    assert!(r.press(Key::ENTER));
    assert_eq!(r.keys(), &[Key::ARROW_UP, Key::ENTER]);
    assert_eq!(r.len(), 2);
    assert!(r.contains(Key::ARROW_UP));
    assert!(r.contains(Key::ENTER));
    assert!(!r.contains(Key::TAB));
}

// T040: D-pad up and left-stick up both bind ARROW_UP; C put it in two slots
// (behaviour change D2: one slot).
#[test]
fn press_duplicate_uses_one_slot() {
    let mut r = KeyboardReport::NEUTRAL;
    assert!(r.press(Key::ARROW_UP));
    assert!(r.press(Key::ARROW_UP));
    assert_eq!(r.keys(), &[Key::ARROW_UP]);
    assert_eq!(r.len(), 1);
}

// duplicates never cost a slot, even when the report is full
#[test]
fn press_duplicate_when_full_is_still_true() {
    let mut r = report_with(Modifiers::EMPTY, &SIX);
    assert!(r.is_full());
    assert!(r.press(Key::TAB));
    assert_eq!(r.keys(), &SIX);
}

// T040 / D2: past six distinct keys the first six win, like C's `if (nKeys < 6)`
#[test]
fn press_seventh_distinct_key_is_dropped() {
    let mut r = report_with(Modifiers::EMPTY, &SIX);
    assert!(!r.press(Key::ARROW_UP));
    assert_eq!(r.keys(), &SIX);
    assert!(!r.contains(Key::ARROW_UP));
    assert_eq!(r.len(), MAX_KEYS);
}

// C skips zero keycodes in a chord (`for (j = 1; j < 7 && b.outData[j]; j++)`)
#[test]
fn press_none_is_a_noop() {
    let mut r = KeyboardReport::NEUTRAL;
    assert!(r.press(Key::NONE));
    assert!(r.is_neutral());
    let mut full = report_with(Modifiers::EMPTY, &SIX);
    assert!(full.press(Key::NONE));
    assert_eq!(full.keys(), &SIX);
}

#[test]
fn contains_none_is_false() {
    let r = report_with(Modifiers::EMPTY, &[Key::ENTER]);
    assert!(!r.contains(Key::NONE));
}

// ---- encode ----

// Steam+X -> Win+Ctrl+O (lizard_map.cpp)
#[test]
fn encode_layout() {
    let r = report_with(Modifiers::LGUI | Modifiers::LCTRL, &[Key::O]);
    assert_eq!(r.encode(), [0x41, 0x09, 0x00, 0x12, 0, 0, 0, 0, 0]);
}

// Steam+L4 -> Ctrl+Alt+Delete
#[test]
fn encode_ctrl_alt_delete() {
    let r = report_with(Modifiers::LCTRL | Modifiers::LALT, &[Key::DELETE]);
    assert_eq!(r.encode(), [0x41, 0x05, 0x00, 0x4C, 0, 0, 0, 0, 0]);
}

#[test]
fn encode_full() {
    let r = report_with(Modifiers::from_bits(0xFF), &SIX);
    assert_eq!(
        r.encode(),
        [0x41, 0xFF, 0x00, 0x28, 0x29, 0x2B, 0x4B, 0x4E, 0x4C]
    );
}

#[test]
fn encode_reserved_byte_is_always_zero() {
    let r = report_with(Modifiers::from_bits(0xFF), &SIX);
    assert_eq!(r.encode()[2], 0);
}

// ---- decode ----

#[test]
fn decode_layout() {
    let r = KeyboardReport::decode(&[0x41, 0x09, 0x00, 0x12, 0, 0, 0, 0, 0]).unwrap();
    assert_eq!(r, report_with(Modifiers::LGUI | Modifiers::LCTRL, &[Key::O]));
}

#[test]
fn decode_rejects_wrong_id() {
    assert_eq!(KeyboardReport::decode(&[0x40, 0, 0, 0, 0, 0, 0, 0, 0]), None);
}

#[test]
fn decode_rejects_every_short_length() {
    let full = [0x41, 0x01, 0x00, 0x28, 0, 0, 0, 0, 0];
    for n in 0..KEYBOARD_REPORT_LEN {
        assert_eq!(KeyboardReport::decode(&full[..n]), None, "len={n}");
    }
}

#[test]
fn decode_ignores_trailing_bytes_and_reserved() {
    let mut buf = [0xAAu8; 64];
    buf[..9].copy_from_slice(&[0x41, 0x00, 0x77, 0x28, 0, 0, 0, 0, 0]);
    assert_eq!(
        KeyboardReport::decode(&buf),
        Some(report_with(Modifiers::EMPTY, &[Key::ENTER]))
    );
}

// a host-style report with gaps or repeats normalises to the packed form
#[test]
fn decode_packs_gaps_and_dedupes() {
    let r = KeyboardReport::decode(&[0x41, 0, 0, 0, 0x28, 0, 0x28, 0x29, 0]).unwrap();
    assert_eq!(r.keys(), &[Key::ENTER, Key::ESCAPE]);
}

#[test]
fn decode_rejects_invalid_key_bytes() {
    for bad in [0x01, 0x03, 0x66, 0xE0, 0xFF] {
        let bytes = [0x41, 0, 0, 0x28, bad, 0, 0, 0, 0];
        assert_eq!(KeyboardReport::decode(&bytes), None, "bad={bad:#04x}");
    }
}

// ---- properties ----

fn arb_key() -> impl Strategy<Value = Key> {
    (Key::USAGE_MIN..=Key::USAGE_MAX).prop_map(|u| Key::from_usage(u).unwrap())
}

proptest! {
    // any sequence of presses: no duplicates, never more than six, first-come order kept
    #[test]
    fn press_invariants(keys in prop::collection::vec(arb_key(), 0..20)) {
        let mut r = KeyboardReport::NEUTRAL;
        let mut expect: Vec<Key> = Vec::new();
        for k in keys {
            let accepted = r.press(k);
            if !expect.contains(&k) && expect.len() < MAX_KEYS {
                expect.push(k);
            }
            prop_assert_eq!(accepted, expect.contains(&k));
        }
        prop_assert_eq!(r.keys(), expect.as_slice());
        prop_assert!(r.len() <= MAX_KEYS);
    }

    #[test]
    fn roundtrip(m in any::<u8>(), keys in prop::collection::vec(arb_key(), 0..8)) {
        let mut r = KeyboardReport::NEUTRAL;
        r.modifiers = Modifiers::from_bits(m);
        for k in keys {
            r.press(k);
        }
        prop_assert_eq!(KeyboardReport::decode(&r.encode()), Some(r));
    }

    // wire contract for anything press() can build
    #[test]
    fn wire_is_always_valid(m in any::<u8>(), keys in prop::collection::vec(arb_key(), 0..8)) {
        let mut r = KeyboardReport::NEUTRAL;
        r.modifiers = Modifiers::from_bits(m);
        for k in keys {
            r.press(k);
        }
        let e = r.encode();
        prop_assert_eq!(e[0], REPORT_ID_KEYBOARD);
        prop_assert_eq!(e[2], 0);
        let slots = &e[3..];
        let used = slots.iter().take_while(|&&b| b != 0).count();
        // packed: no key after the first empty slot
        prop_assert!(slots[used..].iter().all(|&b| b == 0));
        for &b in &slots[..used] {
            prop_assert!((Key::USAGE_MIN..=Key::USAGE_MAX).contains(&b));
        }
    }
}
