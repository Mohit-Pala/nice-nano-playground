// Contract for ropk_proto::report -- one TAG_REPORT TLV value -> typed report.
// Layout: PROTOCOL.md §8. Offsets are ABSOLUTE (report id at 0x00), unlike
// triton.h's s16off/u16off which add 2. Covers T001, T002, T007, T010.

use ropk_proto::input::{Imu, Sticks, Trackpad, Triggers};
use ropk_proto::report::*;
use ropk_proto::sc_controller_buttons::{Button, Buttons};

// ---- fixtures ----

fn put_u16(r: &mut [u8], off: usize, v: u16) {
    r[off..off + 2].copy_from_slice(&v.to_le_bytes());
}
fn put_i16(r: &mut [u8], off: usize, v: i16) {
    r[off..off + 2].copy_from_slice(&v.to_le_bytes());
}
fn put_u32(r: &mut [u8], off: usize, v: u32) {
    r[off..off + 4].copy_from_slice(&v.to_le_bytes());
}

// Every field gets a distinct value, so reading the wrong offset can't pass.
// Literal offsets on purpose: this pins the layout independently of OFF_*.
fn report_45() -> [u8; 46] {
    let mut r = [0u8; 46];
    r[0x00] = 0x45;
    r[0x01] = 0x5A; // seq
    put_u32(&mut r, 0x02, 0x0060_4001); // A | Start(14) | RT-pad touch+click(21,22)
    put_u16(&mut r, 0x06, 0x1234); // LT
    put_u16(&mut r, 0x08, 0x8000); // RT, full pull
    put_i16(&mut r, 0x0A, -1001); // LX
    put_i16(&mut r, 0x0C, 1002); // LY
    put_i16(&mut r, 0x0E, -1003); // RX
    put_i16(&mut r, 0x10, 1004); // RY
    put_i16(&mut r, 0x12, -2001); // L pad x
    put_i16(&mut r, 0x14, 2002); // L pad y
    put_i16(&mut r, 0x16, 2003); // L pad press
    put_i16(&mut r, 0x18, -3001); // R pad x
    put_i16(&mut r, 0x1A, 3002); // R pad y
    put_i16(&mut r, 0x1C, 3003); // R pad press
    put_u32(&mut r, 0x1E, 0xDEAD_BEEF); // IMU timestamp
    put_i16(&mut r, 0x22, 101); // accel x
    put_i16(&mut r, 0x24, -102); // accel y
    put_i16(&mut r, 0x26, 4096); // accel z (~1 g)
    put_i16(&mut r, 0x28, -201); // gyro x
    put_i16(&mut r, 0x2A, 202); // gyro y
    put_i16(&mut r, 0x2C, -203); // gyro z
    r
}

// 0x42: same first 46 bytes, plus 8 trailing (rf_link.cpp:439)
fn report_42() -> [u8; 54] {
    let mut r = [0u8; 54];
    r[..46].copy_from_slice(&report_45());
    r[0] = 0x42;
    put_u16(&mut r, 46, 0x7FFF);
    r
}

fn expected_basic(input: &ropk_proto::input::ControllerInput) {
    assert_eq!(input.buttons.bits(), 0x0060_4001);
    assert!(input.buttons.has(Button::A));
    assert!(input.buttons.has(Button::Start));
    assert!(input.buttons.has(Button::RTrackpadTouch));
    assert!(input.buttons.has(Button::RTrackpadClick));
    assert_eq!(
        input.triggers,
        Triggers {
            l: 0x1234,
            r: 0x8000
        }
    );
    assert_eq!(
        input.sticks,
        Sticks {
            lx: -1001,
            ly: 1002,
            rx: -1003,
            ry: 1004
        }
    );
    assert_eq!(
        input.l_trackpad,
        Trackpad {
            x: -2001,
            y: 2002,
            press: 2003
        }
    );
    assert_eq!(
        input.r_trackpad,
        Trackpad {
            x: -3001,
            y: 3002,
            press: 3003
        }
    );
}

const EXPECTED_IMU: Imu = Imu {
    timestamp: 0xDEAD_BEEF,
    accel: [101, -102, 4096],
    gyro: [-201, 202, -203],
};

// ---- constants ----

#[test]
fn report_ids() {
    assert_eq!(REPORT_ID_INPUT_LEGACY, 0x45);
    assert_eq!(REPORT_ID_INPUT, 0x42);
    assert_eq!(REPORT_ID_STATUS, 0x43);
    assert_eq!(REPORT_ID_STATUS_EVENT, 0x44);
}

#[test]
fn lengths() {
    // 46 = 1 id + 45 body; min stops after the R pad press
    assert_eq!(INPUT_FULL_LEN, 46);
    assert_eq!(INPUT_MIN_LEN, 30);
    assert_eq!(STATUS_MIN_LEN, 3);
}

#[test]
fn offsets_match_protocol_table() {
    let table = [
        (OFF_SEQ, 0x01),
        (OFF_BUTTONS, 0x02),
        (OFF_LT, 0x06),
        (OFF_RT, 0x08),
        (OFF_LX, 0x0A),
        (OFF_LY, 0x0C),
        (OFF_RX, 0x0E),
        (OFF_RY, 0x10),
        (OFF_LPAD_X, 0x12),
        (OFF_LPAD_Y, 0x14),
        (OFF_LPAD_PRESS, 0x16),
        (OFF_RPAD_X, 0x18),
        (OFF_RPAD_Y, 0x1A),
        (OFF_RPAD_PRESS, 0x1C),
        (OFF_IMU_TIMESTAMP, 0x1E),
        (OFF_ACCEL_X, 0x22),
        (OFF_ACCEL_Y, 0x24),
        (OFF_ACCEL_Z, 0x26),
        (OFF_GYRO_X, 0x28),
        (OFF_GYRO_Y, 0x2A),
        (OFF_GYRO_Z, 0x2C),
    ];
    for (i, (got, want)) in table.into_iter().enumerate() {
        assert_eq!(got, want, "row {i}");
    }
    // last field ends exactly at the full length
    assert_eq!(OFF_GYRO_Z + 2, INPUT_FULL_LEN);
    assert_eq!(OFF_RPAD_PRESS + 2, INPUT_MIN_LEN);
}

// ---- T001: full 0x45 ----

#[test]
fn t001_full_report_decodes_every_field() {
    let r = report_45();
    let Ok(Report::Input(rep)) = parse_report(&r) else {
        panic!("expected Input");
    };
    assert_eq!(rep.id, 0x45);
    assert_eq!(rep.seq, 0x5A);
    expected_basic(&rep.input);
    assert_eq!(rep.input.imu, Some(EXPECTED_IMU));
}

#[test]
fn raw_is_the_whole_value_unchanged() {
    let r = report_45();
    let rep = parse_input(&r).unwrap();
    assert_eq!(rep.raw, &r[..]);
    // borrowed, not copied
    assert_eq!(rep.raw.as_ptr(), r.as_ptr());
}

#[test]
fn parse_input_and_parse_report_agree() {
    let r = report_45();
    assert_eq!(
        parse_report(&r),
        Ok(Report::Input(parse_input(&r).unwrap()))
    );
}

// ---- T002: 0x42 ----

#[test]
fn t002_0x42_decodes_like_0x45_and_keeps_tail() {
    let r = report_42();
    let Ok(Report::Input(rep)) = parse_report(&r) else {
        panic!("expected Input");
    };
    assert_eq!(rep.id, 0x42);
    assert_eq!(rep.seq, 0x5A);
    expected_basic(&rep.input);
    assert_eq!(rep.input.imu, Some(EXPECTED_IMU));
    assert_eq!(rep.raw.len(), 54);
    assert_eq!(&rep.raw[46..48], &[0xFF, 0x7F]);
}

#[test]
fn ox42_and_ox45_produce_identical_input() {
    let a = parse_input(&report_45()).unwrap().input;
    let b = parse_input(&report_42()).unwrap().input;
    assert_eq!(a, b);
}

// ---- T007: short reports and IMU validity ----

#[test]
fn t007_min_len_has_input_but_no_imu() {
    let r = report_45();
    let rep = parse_input(&r[..INPUT_MIN_LEN]).unwrap();
    expected_basic(&rep.input);
    assert_eq!(rep.input.imu, None);
    assert_eq!(rep.raw.len(), INPUT_MIN_LEN);
}

#[test]
fn t007_imu_only_when_all_46_bytes_present() {
    let r = report_45();
    for len in INPUT_MIN_LEN..INPUT_FULL_LEN {
        let rep = parse_input(&r[..len]).unwrap();
        expected_basic(&rep.input);
        assert_eq!(rep.input.imu, None, "len={len}: partial IMU must be None");
    }
    assert_eq!(parse_input(&r).unwrap().input.imu, Some(EXPECTED_IMU));
}

#[test]
fn t007_every_length_below_min_is_rejected() {
    let r = report_45();
    assert_eq!(parse_report(&r[..0]), Err(ReportError::Empty));
    for len in 1..INPUT_MIN_LEN {
        assert_eq!(
            parse_report(&r[..len]),
            Err(ReportError::InputTooShort { id: 0x45, len }),
            "len={len}"
        );
    }
}

#[test]
fn short_0x42_reports_its_own_id() {
    let r = report_42();
    assert_eq!(
        parse_report(&r[..10]),
        Err(ReportError::InputTooShort { id: 0x42, len: 10 })
    );
}

#[test]
fn parse_input_empty_is_empty() {
    assert_eq!(parse_input(&[]), Err(ReportError::Empty));
}

// ---- T010: numeric boundaries ----

#[test]
fn t010_axis_extremes_decode_exactly() {
    for v in [i16::MIN, -32767, -1, 0, 1, i16::MAX] {
        let mut r = report_45();
        for off in [0x0A, 0x0C, 0x0E, 0x10, 0x12, 0x14, 0x16, 0x18, 0x1A, 0x1C] {
            put_i16(&mut r, off, v);
        }
        for off in [0x22, 0x24, 0x26, 0x28, 0x2A, 0x2C] {
            put_i16(&mut r, off, v);
        }
        let i = parse_input(&r).unwrap().input;
        assert_eq!(
            i.sticks,
            Sticks {
                lx: v,
                ly: v,
                rx: v,
                ry: v
            }
        );
        assert_eq!(
            i.l_trackpad,
            Trackpad {
                x: v,
                y: v,
                press: v
            }
        );
        assert_eq!(
            i.r_trackpad,
            Trackpad {
                x: v,
                y: v,
                press: v
            }
        );
        let imu = i.imu.unwrap();
        assert_eq!(imu.accel, [v; 3]);
        assert_eq!(imu.gyro, [v; 3]);
    }
}

#[test]
fn t010_triggers_stay_raw_u16() {
    for v in [0, 0x7F, 0x80, 0x7FFF, 0x8000, 0xFFFF] {
        let mut r = report_45();
        put_u16(&mut r, 0x06, v);
        put_u16(&mut r, 0x08, v);
        let t = parse_input(&r).unwrap().input.triggers;
        assert_eq!(t, Triggers { l: v, r: v });
    }
}

#[test]
fn imu_timestamp_full_u32_range() {
    for v in [0, 1, 0x8000_0000, u32::MAX] {
        let mut r = report_45();
        put_u32(&mut r, 0x1E, v);
        assert_eq!(parse_input(&r).unwrap().input.imu.unwrap().timestamp, v);
    }
}

// ---- buttons: no masking in the decoder ----

#[test]
fn all_32_button_bits_kept() {
    // bits 30/31 aren't physical, and 28/29 (grips) are always set on 0x42:
    // masking is the consumer's choice, never the decoder's
    for bits in [0xC000_0001, 0x3000_0000, u32::MAX, 0] {
        let mut r = report_45();
        put_u32(&mut r, 0x02, bits);
        assert_eq!(
            parse_input(&r).unwrap().input.buttons,
            Buttons::from_bits(bits)
        );
    }
}

// ---- no mutation ----

// rf_link.cpp:544-554/617 clears bits directly in the report to hide chords.
// The Rust decoder must leave the bytes exactly as received.
#[test]
fn decoder_never_modifies_input() {
    let mut r = report_45();
    put_u32(
        &mut r,
        0x02,
        Buttons::BACK4.bits() | Button::A.bit() | Button::Steam.bit() | Button::Y.bit(),
    );
    let before = r;
    let rep = parse_report(&r).unwrap();
    assert_eq!(r, before);
    let Report::Input(rep) = rep else { panic!() };
    assert_eq!(rep.raw, &before[..]);
}

// ---- 0x43 status ----

#[test]
fn status_decodes_charge_and_battery() {
    let v = [0x43, 2, 87];
    assert_eq!(
        parse_report(&v),
        Ok(Report::Status(StatusReport {
            charge_state: 2,
            battery_pct: 87,
            raw: &v
        }))
    );
}

#[test]
fn status_keeps_extra_bytes_in_raw() {
    let v = [0x43, 1, 50, 0xAA, 0xBB];
    let Ok(Report::Status(s)) = parse_report(&v) else {
        panic!()
    };
    assert_eq!((s.charge_state, s.battery_pct), (1, 50));
    assert_eq!(s.raw, &v[..]);
}

#[test]
fn status_too_short() {
    assert_eq!(
        parse_report(&[0x43]),
        Err(ReportError::StatusTooShort { len: 1 })
    );
    assert_eq!(
        parse_report(&[0x43, 2]),
        Err(ReportError::StatusTooShort { len: 2 })
    );
    assert_eq!(
        parse_status(&[]),
        Err(ReportError::StatusTooShort { len: 0 })
    );
}

// ---- 0x44 / unknown / empty ----

#[test]
fn status_event_is_passed_through() {
    let v = [0x44, 9, 8, 7];
    assert_eq!(parse_report(&v), Ok(Report::StatusEvent(&v)));
    assert_eq!(parse_report(&[0x44]), Ok(Report::StatusEvent(&[0x44])));
}

// unknown ids are not errors: the firmware logs them in case a controller
// update moves input to a new id (rf_link.cpp:660)
#[test]
fn unknown_ids_are_ok_unknown() {
    let v = [0x99, 1, 2, 3];
    assert_eq!(parse_report(&v), Ok(Report::Unknown { id: 0x99, raw: &v }));
    assert_eq!(
        parse_report(&[0x40]),
        Ok(Report::Unknown {
            id: 0x40,
            raw: &[0x40]
        })
    );
    for id in [0x00, 0x41, 0x46, 0x79, 0x7B, 0xFF] {
        assert!(
            matches!(parse_report(&[id; 50]), Ok(Report::Unknown { .. })),
            "id={id:#04x}"
        );
    }
}

#[test]
fn empty_is_error() {
    assert_eq!(parse_report(&[]), Err(ReportError::Empty));
}

// ---- lifetimes ----

#[test]
fn raw_outlives_the_parse_call() {
    let buf = report_42();
    let raw: &[u8] = {
        let Ok(Report::Input(rep)) = parse_report(&buf) else {
            panic!()
        };
        rep.raw
    };
    assert_eq!(raw.len(), 54);
}

// ---- robustness ----

#[test]
fn arbitrary_bytes_never_panic() {
    let mut seed: u32 = 0xC0FF_EE11;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed as u8
    };
    for len in 0..=96 {
        for first in [0x42, 0x43, 0x44, 0x45, 0x00] {
            let mut buf: Vec<u8> = (0..len).map(|_| next()).collect();
            if let Some(b) = buf.first_mut() {
                *b = first;
            }
            let _ = parse_report(&buf);
        }
    }
}