// Contract for ropk_proto::hid::consumer -- lizard media keys, report 0x03.
// Only declared in the lizard-only descriptor (pure MODE_LIZARD); PUCK_HID_DESC
// has no 0x03, so it must never be sent in Steam mode.
// C: puck_hid.cpp PUCK_LIZARD_HID_DESC tail, lizard_map.h LZ_OUT_CONSUMER
//    (bit0 = vol+, bit1 = vol-), mode_lizard.cpp rfLizard() "consumer control"
//    and rfLizardRelease() (cc = 0).
// Wire: [0x03][bits], bit 0 = Volume Increment (0xE9), bit 1 = Volume Decrement (0xEA).
// Groundwork for T047.

use proptest::prelude::*;
use ropk_proto::hid::consumer::{
    CONSUMER_BODY_LEN, CONSUMER_REPORT_LEN, ConsumerControls, ConsumerReport,
    REPORT_ID_CONSUMER,
};

fn report(controls: ConsumerControls) -> ConsumerReport {
    ConsumerReport { controls }
}

// ---- constants ----

#[test]
fn report_id_is_0x03() {
    assert_eq!(REPORT_ID_CONSUMER, 0x03);
}

#[test]
fn lengths_are_body_1_plus_id() {
    assert_eq!(CONSUMER_BODY_LEN, 1);
    assert_eq!(CONSUMER_REPORT_LEN, 2);
    assert_eq!(CONSUMER_REPORT_LEN, CONSUMER_BODY_LEN + 1);
}

// ---- controls ----

// descriptor order: Usage 0xE9 (Volume Increment) then 0xEA (Volume Decrement);
// C map: addConsumer(0x01, R5, STEAM) = vol up, addConsumer(0x02, L5, STEAM) = vol down
#[test]
fn control_bits_match_descriptor_and_c_map() {
    assert_eq!(ConsumerControls::EMPTY.bits(), 0x00);
    assert_eq!(ConsumerControls::VOL_UP.bits(), 0x01);
    assert_eq!(ConsumerControls::VOL_DOWN.bits(), 0x02);
    assert_eq!(ConsumerControls::ALL.bits(), 0x03);
}

#[test]
fn from_bits_masks_padding_bits() {
    // bits 2..7 are descriptor padding (6-bit Input Const)
    assert_eq!(ConsumerControls::from_bits(0xFF), ConsumerControls::ALL);
    assert_eq!(ConsumerControls::from_bits(0xFC), ConsumerControls::EMPTY);
    assert_eq!(ConsumerControls::from_bits(0x05), ConsumerControls::VOL_UP);
}

#[test]
fn set_operations() {
    let both = ConsumerControls::VOL_UP | ConsumerControls::VOL_DOWN;
    assert_eq!(both, ConsumerControls::ALL);
    assert!(both.contains(ConsumerControls::VOL_UP));
    assert!(both.contains(ConsumerControls::EMPTY));
    assert!(!ConsumerControls::VOL_UP.contains(ConsumerControls::VOL_DOWN));
    assert_eq!(
        ConsumerControls::VOL_UP.with(ConsumerControls::VOL_DOWN),
        ConsumerControls::ALL
    );
    assert_eq!(
        ConsumerControls::VOL_UP.union(ConsumerControls::VOL_DOWN),
        ConsumerControls::ALL
    );
    assert_eq!(
        both.remove(ConsumerControls::VOL_UP),
        ConsumerControls::VOL_DOWN
    );
    assert!(ConsumerControls::EMPTY.is_empty());
    assert!(!both.is_empty());
    assert_eq!(ConsumerControls::default(), ConsumerControls::EMPTY);
}

// ---- neutral ----

#[test]
fn neutral_is_empty() {
    let n = ConsumerReport::NEUTRAL;
    assert_eq!(n, report(ConsumerControls::EMPTY));
    assert!(n.is_neutral());
    assert_eq!(ConsumerReport::default(), n);
}

// C rfLizardRelease sends cc = 0 -- the release that T047 says must not be lost
#[test]
fn neutral_encodes_to_id_and_zero() {
    assert_eq!(ConsumerReport::NEUTRAL.encode(), [0x03, 0x00]);
}

#[test]
fn held_control_is_not_neutral() {
    assert!(!report(ConsumerControls::VOL_UP).is_neutral());
    assert!(!report(ConsumerControls::VOL_DOWN).is_neutral());
}

// ---- encode ----

#[test]
fn encode_each_control() {
    assert_eq!(report(ConsumerControls::VOL_UP).encode(), [0x03, 0x01]);
    assert_eq!(report(ConsumerControls::VOL_DOWN).encode(), [0x03, 0x02]);
    assert_eq!(report(ConsumerControls::ALL).encode(), [0x03, 0x03]);
}

// ---- decode ----

#[test]
fn decode_layout() {
    assert_eq!(
        ConsumerReport::decode(&[0x03, 0x02]),
        Some(report(ConsumerControls::VOL_DOWN))
    );
}

#[test]
fn decode_rejects_wrong_id() {
    assert_eq!(ConsumerReport::decode(&[0x40, 0x01]), None);
    assert_eq!(ConsumerReport::decode(&[0x41, 0x01]), None);
}

#[test]
fn decode_rejects_short() {
    assert_eq!(ConsumerReport::decode(&[]), None);
    assert_eq!(ConsumerReport::decode(&[0x03]), None);
}

#[test]
fn decode_ignores_trailing_bytes_and_masks_padding() {
    let mut buf = [0xAAu8; 64];
    buf[..2].copy_from_slice(&[0x03, 0xFD]);
    assert_eq!(
        ConsumerReport::decode(&buf),
        Some(report(ConsumerControls::VOL_UP))
    );
}

// ---- properties ----

proptest! {
    #[test]
    fn roundtrip(b in 0u8..=3) {
        let r = report(ConsumerControls::from_bits(b));
        prop_assert_eq!(ConsumerReport::decode(&r.encode()), Some(r));
    }

    #[test]
    fn wire_is_always_valid(b in any::<u8>()) {
        let e = report(ConsumerControls::from_bits(b)).encode();
        prop_assert_eq!(e[0], REPORT_ID_CONSUMER);
        prop_assert_eq!(e[1] & !0x03, 0);
    }
}
