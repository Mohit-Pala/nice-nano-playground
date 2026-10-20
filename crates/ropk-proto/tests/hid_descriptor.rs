// Contract for ropk_proto::hid::descriptor -- the two slot HID report descriptors.
//   PUCK_HID_DESC         Steam mode. Byte-for-byte the July C PUCK_HID_DESC (372 B).
//   PUCK_LIZARD_HID_DESC  pure lizard. C's PUCK_LIZARD_HID_DESC (401 B) plus D1:
//                         the mouse declares 3 buttons (middle) instead of 2.
// C: puck_hid.cpp PUCK_HID_DESC / PUCK_LIZARD_HID_DESC @ 096e036.
// Covers T043 "descriptor and report agree" for the lizard reports (0x40, 0x41, 0x03)
// and pins the vendor report sizes Steam depends on.
//
// A tiny HID 1.11 short-item walker lives at the bottom of this file. It only
// understands what these descriptors use (no long items, no push/pop, no delimiters).

use std::collections::BTreeSet;

use ropk_proto::hid::consumer::{CONSUMER_BODY_LEN, REPORT_ID_CONSUMER};
use ropk_proto::hid::descriptor::{PUCK_HID_DESC, PUCK_LIZARD_HID_DESC};
use ropk_proto::hid::keyboard::{KEYBOARD_BODY_LEN, Key, MAX_KEYS, REPORT_ID_KEYBOARD};
use ropk_proto::hid::mouse::{AXIS_MAX, AXIS_MIN, MOUSE_BODY_LEN, REPORT_ID_MOUSE};
use ropk_proto::report::{INPUT_FULL_LEN, REPORT_ID_INPUT, REPORT_ID_INPUT_LEGACY};

const PUCK_LEN: usize = 372;
const LIZARD_LEN: usize = 401;

// D1: the only bytes the lizard descriptor changes inside the shared 372-byte prefix.
// Mouse button block: 19 01 [29 02] 15 00 25 01 75 01 [95 02] 81 02 [75 06] 95 01 81 01
const OFF_BTN_USAGE_MAX: usize = 17; // 29 02 -> 29 03
const OFF_BTN_COUNT: usize = 25; // 95 02 -> 95 03
const OFF_BTN_PAD_SIZE: usize = 29; // 75 06 -> 75 05

// C's consumer collection, appended verbatim after the shared prefix
const CONSUMER_TAIL: [u8; 29] = [
    0x05, 0x0C, 0x09, 0x01, 0xA1, 0x01, 0x85, 0x03, 0x15, 0x00, 0x25, 0x01, 0x09, 0xE9, 0x09,
    0xEA, 0x75, 0x01, 0x95, 0x02, 0x81, 0x02, 0x75, 0x06, 0x95, 0x01, 0x81, 0x01, 0xC0,
];

// vendor reports, (id, body bytes), straight from the C descriptor
const VENDOR_INPUTS: [(u8, u32); 6] = [
    (0x42, 53),
    (0x44, 5),
    (0x79, 1),
    (0x43, 14),
    (0x7B, 12),
    (0x45, 45),
];
const OUTPUTS: [(u8, u32); 10] = [
    (0x80, 9),
    (0x81, 7),
    (0x82, 3),
    (0x83, 9),
    (0x84, 8),
    (0x85, 3),
    (0x86, 3),
    (0x87, 63),
    (0x89, 63),
    (0x88, 63),
];
const FEATURES: [(u8, u32); 2] = [(0x01, 63), (0x02, 63)];

fn both() -> [(&'static str, &'static [u8]); 2] {
    [("puck", PUCK_HID_DESC), ("lizard", PUCK_LIZARD_HID_DESC)]
}

// ---- shape ----

#[test]
fn lengths_match_c() {
    assert_eq!(PUCK_HID_DESC.len(), PUCK_LEN);
    assert_eq!(PUCK_LIZARD_HID_DESC.len(), LIZARD_LEN);
}

#[test]
fn both_parse_with_balanced_collections() {
    for (name, d) in both() {
        parse(d).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}

// Steam mode must stay exactly C's shape: the mouse still says 2 buttons there.
#[test]
fn puck_mouse_block_is_c_verbatim() {
    assert_eq!(&PUCK_HID_DESC[OFF_BTN_USAGE_MAX - 1..=OFF_BTN_USAGE_MAX], &[0x29, 0x02]);
    assert_eq!(&PUCK_HID_DESC[OFF_BTN_COUNT - 1..=OFF_BTN_COUNT], &[0x95, 0x02]);
    assert_eq!(&PUCK_HID_DESC[OFF_BTN_PAD_SIZE - 1..=OFF_BTN_PAD_SIZE], &[0x75, 0x06]);
}

// lizard = puck with exactly the three D1 bytes changed, then the consumer tail
#[test]
fn lizard_is_puck_plus_d1_plus_consumer_tail() {
    let (prefix, tail) = PUCK_LIZARD_HID_DESC.split_at(PUCK_LEN);
    assert_eq!(tail, &CONSUMER_TAIL);

    let diffs: Vec<usize> = (0..PUCK_LEN)
        .filter(|&i| prefix[i] != PUCK_HID_DESC[i])
        .collect();
    assert_eq!(diffs, vec![OFF_BTN_USAGE_MAX, OFF_BTN_COUNT, OFF_BTN_PAD_SIZE]);
    assert_eq!(prefix[OFF_BTN_USAGE_MAX], 0x03);
    assert_eq!(prefix[OFF_BTN_COUNT], 0x03);
    assert_eq!(prefix[OFF_BTN_PAD_SIZE], 0x05);
}

// ---- which reports exist ----

#[test]
fn input_report_ids() {
    let puck: BTreeSet<u8> = [0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x79, 0x7B].into();
    let mut lizard = puck.clone();
    lizard.insert(0x03);
    assert_eq!(ids(&parse(PUCK_HID_DESC).unwrap(), Kind::Input), puck);
    assert_eq!(ids(&parse(PUCK_LIZARD_HID_DESC).unwrap(), Kind::Input), lizard);
}

// 0x03 is lizard-only: sending it in Steam mode is a report the host never heard of
#[test]
fn consumer_only_in_lizard() {
    let puck = parse(PUCK_HID_DESC).unwrap();
    assert_eq!(bits(&puck, Kind::Input, REPORT_ID_CONSUMER), 0);
}

#[test]
fn output_and_feature_ids_are_shared() {
    for (name, d) in both() {
        let f = parse(d).unwrap();
        let outs: BTreeSet<u8> = OUTPUTS.iter().map(|&(id, _)| id).collect();
        let feats: BTreeSet<u8> = FEATURES.iter().map(|&(id, _)| id).collect();
        assert_eq!(ids(&f, Kind::Output), outs, "{name}");
        assert_eq!(ids(&f, Kind::Feature), feats, "{name}");
    }
}

// ---- T043: descriptor sizes == encoder sizes ----

#[test]
fn mouse_size_matches_encoder() {
    for (name, d) in both() {
        let f = parse(d).unwrap();
        assert_eq!(bits(&f, Kind::Input, REPORT_ID_MOUSE), MOUSE_BODY_LEN as u32 * 8, "{name}");
    }
}

#[test]
fn keyboard_size_matches_encoder() {
    for (name, d) in both() {
        let f = parse(d).unwrap();
        assert_eq!(
            bits(&f, Kind::Input, REPORT_ID_KEYBOARD),
            KEYBOARD_BODY_LEN as u32 * 8,
            "{name}"
        );
    }
}

#[test]
fn consumer_size_matches_encoder() {
    let f = parse(PUCK_LIZARD_HID_DESC).unwrap();
    assert_eq!(
        bits(&f, Kind::Input, REPORT_ID_CONSUMER),
        CONSUMER_BODY_LEN as u32 * 8
    );
}

// ---- mouse details ----

fn mouse_buttons(fields: &[Field]) -> &Field {
    fields
        .iter()
        .find(|f| f.kind == Kind::Input && f.report_id == REPORT_ID_MOUSE && f.usage_page == 0x09)
        .expect("mouse button field")
}

// T043 / D1: every button the encoder can send is declared in lizard mode
#[test]
fn lizard_mouse_declares_three_buttons() {
    let f = parse(PUCK_LIZARD_HID_DESC).unwrap();
    let b = mouse_buttons(&f);
    assert_eq!((b.usage_min, b.usage_max), (Some(1), Some(3)));
    assert_eq!((b.size, b.count), (1, 3));
    assert!(!b.constant);
}

// documents the C bug D1 fixes: middle (bit 2) is padding in Steam mode
#[test]
fn puck_mouse_declares_two_buttons() {
    let f = parse(PUCK_HID_DESC).unwrap();
    let b = mouse_buttons(&f);
    assert_eq!((b.usage_min, b.usage_max), (Some(1), Some(2)));
    assert_eq!((b.size, b.count), (1, 2));
}

#[test]
fn mouse_axes_are_relative_and_match_axis_range() {
    for (name, d) in both() {
        let f = parse(d).unwrap();
        let axes: Vec<&Field> = f
            .iter()
            .filter(|x| x.kind == Kind::Input && x.report_id == REPORT_ID_MOUSE && x.size == 8)
            .collect();
        // X, Y (one field), wheel, pan
        let usages: Vec<(u16, u32)> = axes
            .iter()
            .flat_map(|x| x.usages.iter().map(move |&u| (x.usage_page, u)))
            .collect();
        assert_eq!(usages, vec![(0x01, 0x30), (0x01, 0x31), (0x01, 0x38), (0x0C, 0x238)], "{name}");
        for a in axes {
            assert!(a.relative, "{name}: {a:?}");
            assert_eq!(a.logical_min, AXIS_MIN as i32, "{name}");
            assert_eq!(a.logical_max, AXIS_MAX as i32, "{name}");
        }
    }
}

// ---- keyboard details ----

#[test]
fn keyboard_modifiers_and_key_array_match_encoder() {
    for (name, d) in both() {
        let f = parse(d).unwrap();
        let kb: Vec<&Field> = f
            .iter()
            .filter(|x| x.kind == Kind::Input && x.report_id == REPORT_ID_KEYBOARD)
            .collect();
        assert_eq!(kb.len(), 3, "{name}: modifiers, reserved, keys");

        let (mods, reserved, keys) = (kb[0], kb[1], kb[2]);
        assert_eq!((mods.usage_min, mods.usage_max), (Some(0xE0), Some(0xE7)), "{name}");
        assert_eq!((mods.size, mods.count), (1, 8), "{name}");
        assert!(reserved.constant && reserved.size * reserved.count == 8, "{name}");

        assert!(keys.array, "{name}: key slots are an array");
        assert_eq!((keys.size, keys.count), (8, MAX_KEYS as u32), "{name}");
        assert_eq!(keys.logical_max, Key::USAGE_MAX as i32, "{name}");
        assert_eq!(keys.usage_max, Some(Key::USAGE_MAX as u32), "{name}");
    }
}

// ---- consumer details ----

#[test]
fn consumer_usages_in_bit_order() {
    let f = parse(PUCK_LIZARD_HID_DESC).unwrap();
    let c = f
        .iter()
        .find(|x| x.kind == Kind::Input && x.report_id == REPORT_ID_CONSUMER && !x.constant)
        .unwrap();
    assert_eq!(c.usage_page, 0x0C);
    // bit 0 = Volume Increment, bit 1 = Volume Decrement
    assert_eq!(c.usages, vec![0xE9, 0xEA]);
    assert_eq!((c.size, c.count), (1, 2));
}

// ---- vendor / Steam reports (must never drift) ----

#[test]
fn vendor_input_sizes() {
    for (name, d) in both() {
        let f = parse(d).unwrap();
        for (id, body) in VENDOR_INPUTS {
            assert_eq!(bits(&f, Kind::Input, id), body * 8, "{name} input {id:#04x}");
        }
    }
}

// the controller reports we forward in Steam mode fit their declared size
#[test]
fn forwarded_input_reports_match_report_rs() {
    let f = parse(PUCK_HID_DESC).unwrap();
    assert_eq!(
        bits(&f, Kind::Input, REPORT_ID_INPUT_LEGACY),
        (INPUT_FULL_LEN as u32 - 1) * 8
    );
    assert_eq!(bits(&f, Kind::Input, REPORT_ID_INPUT), 53 * 8);
}

#[test]
fn output_and_feature_sizes() {
    for (name, d) in both() {
        let f = parse(d).unwrap();
        for (id, body) in OUTPUTS {
            assert_eq!(bits(&f, Kind::Output, id), body * 8, "{name} output {id:#04x}");
        }
        for (id, body) in FEATURES {
            assert_eq!(bits(&f, Kind::Feature, id), body * 8, "{name} feature {id:#04x}");
        }
    }
}

// ---- general hygiene ----

// every report is whole bytes and, with its id, fits the 64-byte endpoint
#[test]
fn every_report_is_byte_aligned_and_fits_64() {
    for (name, d) in both() {
        let f = parse(d).unwrap();
        for kind in [Kind::Input, Kind::Output, Kind::Feature] {
            for id in ids(&f, kind) {
                let b = bits(&f, kind, id);
                assert_eq!(b % 8, 0, "{name} {kind:?} {id:#04x}: {b} bits");
                let with_id = b / 8 + 1;
                assert!(with_id <= 64, "{name} {kind:?} {id:#04x}: {with_id} bytes");
            }
        }
    }
}

// every report sits under an explicit Report ID
#[test]
fn no_report_without_id() {
    for (name, d) in both() {
        let f = parse(d).unwrap();
        assert!(f.iter().all(|x| x.report_id != 0), "{name}");
    }
}

// =====================================================================
// Minimal HID 1.11 report-descriptor walker (6.2.2). Test-only.
// =====================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Input,
    Output,
    Feature,
}

#[derive(Clone, Debug)]
struct Field {
    kind: Kind,
    report_id: u8,
    usage_page: u16,
    usages: Vec<u32>,
    usage_min: Option<u32>,
    usage_max: Option<u32>,
    logical_min: i32,
    logical_max: i32,
    size: u32,
    count: u32,
    constant: bool, // data bit 0
    array: bool,    // data bit 1 clear
    relative: bool, // data bit 2
}

#[derive(Default)]
struct Globals {
    usage_page: u16,
    logical_min: i32,
    logical_max: i32,
    size: u32,
    count: u32,
    report_id: u8,
}

#[derive(Default)]
struct Locals {
    usages: Vec<u32>,
    usage_min: Option<u32>,
    usage_max: Option<u32>,
}

fn parse(d: &[u8]) -> Result<Vec<Field>, String> {
    let mut g = Globals::default();
    let mut l = Locals::default();
    let mut depth = 0i32;
    let mut out = Vec::new();
    let mut i = 0;

    while i < d.len() {
        let prefix = d[i];
        if prefix == 0xFE {
            return Err(format!("long item at {i}"));
        }
        let len = [0, 1, 2, 4][(prefix & 3) as usize];
        let typ = (prefix >> 2) & 3;
        let tag = prefix >> 4;
        let data = d
            .get(i + 1..i + 1 + len)
            .ok_or_else(|| format!("item at {i} runs past the end"))?;
        let u = data.iter().rev().fold(0u32, |acc, &b| (acc << 8) | b as u32);
        let s = match len {
            1 => data[0] as i8 as i32,
            2 => i16::from_le_bytes([data[0], data[1]]) as i32,
            4 => u as i32,
            _ => 0,
        };

        match (typ, tag) {
            // main
            (0, 0x8) | (0, 0x9) | (0, 0xB) => {
                let kind = match tag {
                    0x8 => Kind::Input,
                    0x9 => Kind::Output,
                    _ => Kind::Feature,
                };
                out.push(Field {
                    kind,
                    report_id: g.report_id,
                    usage_page: g.usage_page,
                    usages: std::mem::take(&mut l.usages),
                    usage_min: l.usage_min,
                    usage_max: l.usage_max,
                    logical_min: g.logical_min,
                    logical_max: g.logical_max,
                    size: g.size,
                    count: g.count,
                    constant: u & 1 != 0,
                    array: u & 2 == 0,
                    relative: u & 4 != 0,
                });
                l = Locals::default();
            }
            (0, 0xA) => {
                depth += 1;
                l = Locals::default();
            }
            (0, 0xC) => {
                depth -= 1;
                if depth < 0 {
                    return Err(format!("End Collection without Collection at {i}"));
                }
            }
            // global
            (1, 0x0) => g.usage_page = u as u16,
            (1, 0x1) => g.logical_min = s,
            (1, 0x2) => {
                // a 1-byte 0x65 or 0xFF max is unsigned when min >= 0 (HID 6.2.2.7)
                g.logical_max = if g.logical_min >= 0 { u as i32 } else { s }
            }
            (1, 0x7) => g.size = u,
            (1, 0x8) => g.report_id = u as u8,
            (1, 0x9) => g.count = u,
            (1, 0xA) | (1, 0xB) => return Err(format!("push/pop at {i} not supported")),
            // local
            (2, 0x0) => l.usages.push(u),
            (2, 0x1) => l.usage_min = Some(u),
            (2, 0x2) => l.usage_max = Some(u),
            _ => return Err(format!("unknown item {prefix:#04x} at {i}")),
        }
        i += 1 + len;
    }

    if depth != 0 {
        return Err(format!("{depth} collection(s) left open"));
    }
    Ok(out)
}

fn bits(fields: &[Field], kind: Kind, id: u8) -> u32 {
    fields
        .iter()
        .filter(|f| f.kind == kind && f.report_id == id)
        .map(|f| f.size * f.count)
        .sum()
}

fn ids(fields: &[Field], kind: Kind) -> BTreeSet<u8> {
    fields
        .iter()
        .filter(|f| f.kind == kind)
        .map(|f| f.report_id)
        .collect()
}
