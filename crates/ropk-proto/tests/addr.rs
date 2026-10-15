// Contract for ropk_proto::addr -- discovery address, register packing and
// per-bond session address generation.
// Source: radio.cpp:4-76, 100-120; radio.h:24-65; PROTOCOL.md §4.1, §5, §6.
// Session vectors were produced by a Python port of rfGenSessionAddr. Covers T012.

use ropk_proto::addr::*;
use ropk_proto::bond::UUID_LEN;

const IBEX: [u8; 4] = [0x69, 0x62, 0x65, 0x78];

fn sess(uuid: [u8; UUID_LEN], dev: [u32; 2]) -> RadioAddr {
    RadioAddr::session_addr(&uuid, dev)
}

fn addr(base: [u8; 4], prefix: u8) -> RadioAddr {
    RadioAddr { base, prefix }
}

// Straight port of C rfBitrev8 (radio.cpp:100), as an independent reference.
fn c_bitrev8(mut x: u8) -> u8 {
    let mut y = 0u8;
    for _ in 0..8 {
        y = (y << 1) | (x & 1);
        x >>= 1;
    }
    y
}

// Straight port of C rfSetAddr's register math (radio.cpp:110-117).
fn c_regs(a: RadioAddr) -> AddrRegs {
    let b = a.base.map(c_bitrev8);
    AddrRegs {
        base0: (b[0] as u32) << 24 | (b[1] as u32) << 16 | (b[2] as u32) << 8 | b[3] as u32,
        prefix0: c_bitrev8(a.prefix) as u32,
    }
}

// Only uuid[6] is set: every multiplied term is 0, so h = 1 ^ dev0 and
// h2 = dev1. Lets a test steer the hash output exactly.
const STEER: [u8; UUID_LEN] = [0, 0, 0, 0, 0, 0, 1, 0];

// ---- constants ----

#[test]
fn discovery_constants() {
    assert_eq!(DISCOVERY_ADDR, addr(IBEX, 0x10));
    assert_eq!(&DISCOVERY_ADDR.base, b"ibex");
    assert_eq!(DISCOVERY_CHANNEL, 2);
    assert_eq!(DEFAULT_SESSION_CHANNEL, 18);
}

#[test]
fn scrub_constants_match_c() {
    assert_eq!(BASE_BYTE_XOR, 0x5A); // radio.cpp:62
    assert_eq!(PREFIX_REPLACE, 0x5C); // radio.cpp:67
    assert_eq!(DISCOVERY_COLLISION, 0x80); // radio.cpp:75
    assert_eq!(PREFIX_SHIFT, 16);
}

#[test]
fn hash_constants_match_c() {
    assert_eq!(HASH_GOLDEN, 0x9E37_79B1);
    assert_eq!(HASH_MURMUR_1, 0x85EB_CA6B);
    assert_eq!(HASH_MURMUR_2, 0xC2B2_AE35);
    assert_eq!(HASH_XXHASH_1, 0x27D4_EB2F);
    assert_eq!(HASH_XXHASH_2, 0x1656_67B1);
}

// the replacement values must not themselves be degenerate
#[test]
fn scrub_outputs_are_valid_bytes() {
    assert!(!is_invalid_addr_byte(INVALID_BYTE_00 ^ BASE_BYTE_XOR));
    assert!(!is_invalid_addr_byte(INVALID_BYTE_FF ^ BASE_BYTE_XOR));
    assert!(!is_invalid_addr_byte(PREFIX_REPLACE));
    assert!(!is_invalid_addr_byte(IBEX[0] ^ DISCOVERY_COLLISION));
}

#[test]
fn invalid_byte_predicate() {
    assert!(is_invalid_addr_byte(0x00));
    assert!(is_invalid_addr_byte(0xFF));
    for b in [0x01, 0x5A, 0x5C, 0x7F, 0x80, 0xA5, 0xFE] {
        assert!(!is_invalid_addr_byte(b), "{b:#04x}");
    }
    let invalid = (0..=255u8).filter(|&b| is_invalid_addr_byte(b)).count();
    assert_eq!(invalid, 2);
}

// ---- bitrev8 ----

#[test]
fn bitrev8_known_values() {
    let table = [
        (0x00, 0x00),
        (0x01, 0x80),
        (0x80, 0x01),
        (0xFF, 0xFF),
        (0x0F, 0xF0),
        (0xA5, 0xA5),
        (0x10, 0x08),
        (0x69, 0x96),
        (0x62, 0x46),
        (0x65, 0xA6),
        (0x78, 0x1E),
    ];
    for (x, want) in table {
        assert_eq!(bitrev8(x), want, "{x:#04x}");
    }
}

#[test]
fn bitrev8_matches_c_for_all_bytes() {
    for x in 0..=255u8 {
        assert_eq!(bitrev8(x), c_bitrev8(x), "{x:#04x}");
    }
}

#[test]
fn bitrev8_is_its_own_inverse() {
    for x in 0..=255u8 {
        assert_eq!(bitrev8(bitrev8(x)), x);
    }
}

#[test]
fn bitrev8_is_const() {
    const { assert!(bitrev8(0x01) == 0x80) };
}

// ---- to_regs ----

// radio.h:64 documents BASE0 for "ibex". PREFIX0 is 0x08, not the 0x10 that
// comment claims: rfSetAddr bit-reverses the prefix unless g_prefixRaw.
#[test]
fn discovery_regs() {
    let r = DISCOVERY_ADDR.to_regs();
    assert_eq!(r.base0, 0x9646_A61E);
    assert_eq!(r.prefix0, 0x08);
}

#[test]
fn to_regs_is_const() {
    const R: AddrRegs = DISCOVERY_ADDR.to_regs();
    assert_eq!(
        R,
        AddrRegs {
            base0: 0x9646_A61E,
            prefix0: 0x08
        }
    );
}

#[test]
fn base0_is_big_endian_packing_of_reversed_bytes() {
    // base[0] lands in the top byte of BASE0
    let r = addr([0x01, 0x00, 0x00, 0x00], 0x00).to_regs();
    assert_eq!(r.base0, 0x8000_0000);
    let r = addr([0x00, 0x00, 0x00, 0x01], 0x00).to_regs();
    assert_eq!(r.base0, 0x0000_0080);
}

#[test]
fn prefix0_only_uses_low_byte() {
    for p in [0x00, 0x01, 0x10, 0x80, 0xFF] {
        assert!(addr(IBEX, p).to_regs().prefix0 <= 0xFF);
    }
}

#[test]
fn to_regs_matches_c_rfsetaddr() {
    let samples = [
        DISCOVERY_ADDR,
        addr([0x86, 0x17, 0xBD, 0xD0], 0xF7),
        addr([0x5A, 0xA5, 0x5A, 0xA5], 0x5C),
        addr([0x00, 0xFF, 0x0F, 0xF0], 0x01),
    ];
    for a in samples {
        assert_eq!(a.to_regs(), c_regs(a), "{a:?}");
    }
}

// ---- session_addr: vectors from the C algorithm ----

#[test]
fn session_vectors_match_c() {
    let table: [([u8; UUID_LEN], [u32; 2], [u8; 4], u8); 6] = [
        // uuid path, no device id
        (
            [1, 2, 3, 4, 5, 6, 7, 8],
            [0, 0],
            [0xFE, 0x41, 0x89, 0xC2],
            0x4B,
        ),
        // device id mixed in
        (
            [1, 2, 3, 4, 5, 6, 7, 8],
            [0x1234_5678, 0x9ABC_DEF0],
            [0x86, 0x17, 0xBD, 0xD0],
            0xF7,
        ),
        // high uuid bytes: products wrap (needs wrapping_mul)
        (
            [0xAA, 0xBB, 0xCC, 0xDD, 0x11, 0x22, 0x33, 0x44],
            [0xDEAD_BEEF, 0xCAFE_F00D],
            [0x97, 0xAD, 0x43, 0x94],
            0x52,
        ),
        // all-zero uuid: device-id-only fallback
        (
            [0; 8],
            [0x1234_5678, 0x9ABC_DEF0],
            [0x08, 0x5E, 0x6A, 0x6C],
            0x1E,
        ),
        // fallback with zero device id: every byte scrubbed
        ([0; 8], [0, 0], [0x5A, 0x5A, 0x5A, 0x5A], 0x5C),
        // h = 0x11223344, h2 = 0x12AB3456: prefix is bits 16..24 of h2
        (
            STEER,
            [0x1122_3345, 0x12AB_3456],
            [0x44, 0x33, 0x22, 0x11],
            0xAB,
        ),
    ];
    for (i, (uuid, dev, base, prefix)) in table.into_iter().enumerate() {
        assert_eq!(sess(uuid, dev), addr(base, prefix), "row {i}");
    }
}

// two OpenPucks bonded to the same controller must not share an address
#[test]
fn device_id_separates_pucks() {
    let uuid = [1, 2, 3, 4, 5, 6, 7, 8];
    let a = sess(uuid, [0x1234_5678, 0x9ABC_DEF0]);
    let b = sess(uuid, [0x1234_5679, 0x9ABC_DEF0]);
    assert_eq!(b, addr([0x87, 0x17, 0xBD, 0xD0], 0xF7));
    assert_ne!(a, b);
}

// two controllers on the same puck must not share an address
#[test]
fn uuid_separates_slots() {
    let dev = [0x1234_5678, 0x9ABC_DEF0];
    assert_ne!(
        sess([1, 2, 3, 4, 5, 6, 7, 8], dev),
        sess([1, 2, 3, 4, 5, 6, 7, 9], dev)
    );
}

#[test]
fn deterministic() {
    let uuid = [0xAA, 0xBB, 0xCC, 0xDD, 0x11, 0x22, 0x33, 0x44];
    let dev = [0xDEAD_BEEF, 0xCAFE_F00D];
    assert_eq!(sess(uuid, dev), sess(uuid, dev));
}

// ---- session_addr: scrub ----

#[test]
fn base_byte_00_becomes_5a() {
    // h = 0x11223300
    let a = sess(STEER, [0x1122_3301, 0x0034_0000]);
    assert_eq!(a, addr([0x5A, 0x33, 0x22, 0x11], 0x34));
}

#[test]
fn base_byte_ff_becomes_a5() {
    // h = 0x112233FF: XOR, so 0xFF -> 0xA5 (not replaced with 0x5A)
    let a = sess(STEER, [0x1122_33FE, 0x0034_0000]);
    assert_eq!(a, addr([0xA5, 0x33, 0x22, 0x11], 0x34));
}

#[test]
fn every_base_byte_is_scrubbed_independently() {
    // h = 0xFF00FF00
    let a = sess(STEER, [0xFF00_FF01, 0x0034_0000]);
    assert_eq!(a.base, [0x5A, 0xA5, 0x5A, 0xA5]);
}

#[test]
fn prefix_00_is_replaced() {
    let a = sess(STEER, [0x1122_3345, 0x0000_0000]);
    assert_eq!(a.prefix, 0x5C);
}

#[test]
fn prefix_ff_is_replaced_not_xored() {
    // 0xFF ^ 0x5C would be 0xA3: the prefix is REPLACED, unlike base bytes
    let a = sess(STEER, [0x1122_3345, 0x00FF_1234]);
    assert_eq!(a, addr([0x44, 0x33, 0x22, 0x11], 0x5C));
}

#[test]
fn prefix_ignores_h2_bits_outside_16_to_24() {
    let a = sess(STEER, [0x1122_3345, 0x00AB_0000]);
    let b = sess(STEER, [0x1122_3345, 0xFFAB_FFFF]);
    assert_eq!(a.prefix, 0xAB);
    assert_eq!(a, b);
}

// ---- session_addr: discovery guard ----

#[test]
fn hash_landing_on_ibex_is_flipped() {
    // h = 0x78656269 -> LE bytes 69 62 65 78 = "ibex"
    let a = sess(STEER, [0x7865_6268, 0x0034_0000]);
    assert_eq!(a, addr([0xE9, 0x62, 0x65, 0x78], 0x34));
    assert_ne!(a.base, DISCOVERY_ADDR.base);
}

#[test]
fn near_ibex_is_left_alone() {
    // h = 0x7865626A: one bit off "ibex", must not be flipped
    let a = sess(STEER, [0x7865_626B, 0x0034_0000]);
    assert_eq!(a.base, [0x6A, 0x62, 0x65, 0x78]);
}

// ---- C parity quirk ----

// uuid[5] is XORed into h2 bits 0..8, but only bits 16..24 of h2 survive
// (the prefix). So uuid[5] never affects the address. Pinned so a "fix" that
// breaks C parity is a deliberate choice, not an accident.
#[test]
fn uuid_byte_5_has_no_effect_like_c() {
    let dev = [0x1234_5678, 0x9ABC_DEF0];
    let reference = sess([1, 2, 3, 4, 5, 6, 7, 8], dev);
    for v in 0..=255u8 {
        assert_eq!(
            sess([1, 2, 3, 4, 5, v, 7, 8], dev),
            reference,
            "uuid[5]={v}"
        );
    }
}

#[test]
fn every_other_uuid_byte_matters() {
    let dev = [0x1234_5678, 0x9ABC_DEF0];
    let base = [1u8, 2, 3, 4, 5, 6, 7, 8];
    let reference = sess(base, dev);
    for i in [0, 1, 2, 3, 4, 6, 7] {
        let mut u = base;
        u[i] ^= 0x40;
        assert_ne!(sess(u, dev), reference, "uuid[{i}]");
    }
}

// ---- properties over many inputs ----

fn xorshift(seed: &mut u32) -> u32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    *seed
}

#[test]
fn never_degenerate_and_never_discovery() {
    let mut seed = 0x00DD_BA11u32;
    for i in 0..20_000 {
        let mut uuid = [0u8; UUID_LEN];
        // every 8th case exercises the all-zero fallback path
        if i % 8 != 0 {
            for b in &mut uuid {
                *b = xorshift(&mut seed) as u8;
            }
        }
        let dev = [xorshift(&mut seed), xorshift(&mut seed)];
        let a = sess(uuid, dev);
        assert_ne!(a.base, DISCOVERY_ADDR.base, "{uuid:02x?} {dev:08x?}");
        for b in a.base {
            assert!(!is_invalid_addr_byte(b), "{uuid:02x?} {dev:08x?} -> {a:?}");
        }
        assert!(
            !is_invalid_addr_byte(a.prefix),
            "{uuid:02x?} {dev:08x?} -> {a:?}"
        );
    }
}

#[test]
fn session_regs_round_trip_through_c_packing() {
    let mut seed = 0xFEED_5EEDu32;
    for _ in 0..1_000 {
        let mut uuid = [0u8; UUID_LEN];
        for b in &mut uuid {
            *b = xorshift(&mut seed) as u8;
        }
        let a = sess(uuid, [xorshift(&mut seed), xorshift(&mut seed)]);
        assert_eq!(a.to_regs(), c_regs(a));
    }
}