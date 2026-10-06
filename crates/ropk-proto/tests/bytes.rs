// Contract for ropk_proto::bytes -- checked little-endian readers that replace
// triton.h's s16off / u16off / btnsOf. Offsets here are ABSOLUTE (from the
// start of the slice), unlike the C helpers which silently add 2.

use ropk_proto::bytes::{array, i16_le, u16_le, u32_le};

// ---- array ----

#[test]
fn array_reads_n_bytes_at_offset() {
    let b = [0x10, 0x20, 0x30, 0x40, 0x50];
    assert_eq!(array::<3>(&b, 1), Some([0x20, 0x30, 0x40]));
}

#[test]
fn array_exact_fit_at_end() {
    let b = [1, 2, 3, 4];
    assert_eq!(array::<2>(&b, 2), Some([3, 4]));
    assert_eq!(array::<4>(&b, 0), Some([1, 2, 3, 4]));
}

#[test]
fn array_one_past_end_is_none() {
    let b = [1, 2, 3, 4];
    assert_eq!(array::<2>(&b, 3), None);
    assert_eq!(array::<5>(&b, 0), None);
}

#[test]
fn array_offset_past_end_is_none() {
    let b = [1, 2, 3, 4];
    assert_eq!(array::<1>(&b, 4), None);
    assert_eq!(array::<1>(&b, 100), None);
}

#[test]
fn array_zero_len_is_some_up_to_end_only() {
    let b = [1, 2, 3];
    assert_eq!(array::<0>(&b, 3), Some([]));
    assert_eq!(array::<0>(&b, 4), None);
}

// off + N must not overflow / wrap around and succeed
#[test]
fn array_huge_offset_does_not_overflow() {
    let b = [0u8; 8];
    assert_eq!(array::<2>(&b, usize::MAX), None);
    assert_eq!(array::<2>(&b, usize::MAX - 1), None);
}

// ---- u16_le ----

#[test]
fn u16_le_is_little_endian() {
    assert_eq!(u16_le(&[0x34, 0x12], 0), Some(0x1234));
}

#[test]
fn u16_le_full_range() {
    assert_eq!(u16_le(&[0x00, 0x00], 0), Some(0));
    assert_eq!(u16_le(&[0xFF, 0xFF], 0), Some(0xFFFF));
    // a full trigger pull sits near half-scale (triton.h trigU8 comment)
    assert_eq!(u16_le(&[0x00, 0x80], 0), Some(0x8000));
}

#[test]
fn u16_le_at_offset() {
    assert_eq!(u16_le(&[0xAA, 0xBB, 0x01, 0x02], 2), Some(0x0201));
}

#[test]
fn u16_le_out_of_bounds_is_none() {
    assert_eq!(u16_le(&[], 0), None);
    assert_eq!(u16_le(&[0x01], 0), None);
    assert_eq!(u16_le(&[0x01, 0x02], 1), None);
    assert_eq!(u16_le(&[0x01, 0x02], usize::MAX), None);
}

// ---- i16_le ----

// T010 boundaries: -32768, -32767, -1, 0, 1, 32767
#[test]
fn i16_le_sign_boundaries() {
    assert_eq!(i16_le(&[0x00, 0x80], 0), Some(-32768));
    assert_eq!(i16_le(&[0x01, 0x80], 0), Some(-32767));
    assert_eq!(i16_le(&[0xFF, 0xFF], 0), Some(-1));
    assert_eq!(i16_le(&[0x00, 0x00], 0), Some(0));
    assert_eq!(i16_le(&[0x01, 0x00], 0), Some(1));
    assert_eq!(i16_le(&[0xFF, 0x7F], 0), Some(32767));
}

#[test]
fn i16_le_out_of_bounds_is_none() {
    assert_eq!(i16_le(&[], 0), None);
    assert_eq!(i16_le(&[0xFF], 0), None);
    assert_eq!(i16_le(&[0x00, 0x80], 1), None);
}

// ---- u32_le ----

#[test]
fn u32_le_is_little_endian() {
    assert_eq!(u32_le(&[0x78, 0x56, 0x34, 0x12], 0), Some(0x1234_5678));
}

#[test]
fn u32_le_high_bit_stays_unsigned() {
    // bits 28/29 = grip touch, always set on 0x42 reports
    assert_eq!(u32_le(&[0x00, 0x00, 0x00, 0x30], 0), Some(0x3000_0000));
    assert_eq!(u32_le(&[0xFF, 0xFF, 0xFF, 0xFF], 0), Some(u32::MAX));
}

#[test]
fn u32_le_exact_fit_and_one_past() {
    let b = [0u8; 4];
    assert_eq!(u32_le(&b, 0), Some(0));
    assert_eq!(u32_le(&b, 1), None);
    assert_eq!(u32_le(&b[..3], 0), None);
}

// ---- against report 0x45 layout (PROTOCOL.md §8) ----

// Every field has a distinct value so a wrong offset can't pass by accident.
fn report_45() -> [u8; 46] {
    let mut r = [0u8; 46];
    r[0x00] = 0x45; // report id
    r[0x01] = 0x07; // seq
    r[0x02..0x06].copy_from_slice(&0x0020_0001u32.to_le_bytes()); // A + RPadTouch
    r[0x06..0x08].copy_from_slice(&0x1234u16.to_le_bytes()); // LT
    r[0x08..0x0A].copy_from_slice(&0x8000u16.to_le_bytes()); // RT
    r[0x0A..0x0C].copy_from_slice(&(-12000i16).to_le_bytes()); // LX
    r[0x0C..0x0E].copy_from_slice(&12001i16.to_le_bytes()); // LY
    r[0x18..0x1A].copy_from_slice(&(-1i16).to_le_bytes()); // RPX
    r[0x1E..0x22].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes()); // IMU ts
    r[0x2C..0x2E].copy_from_slice(&i16::MIN.to_le_bytes()); // gyro Z (last field)
    r
}

#[test]
fn reads_report_45_fields_at_absolute_offsets() {
    let r = report_45();
    assert_eq!(u32_le(&r, 0x02), Some(0x0020_0001));
    assert_eq!(u16_le(&r, 0x06), Some(0x1234));
    assert_eq!(u16_le(&r, 0x08), Some(0x8000));
    // C: s16off(rep, 8) -- the hidden +2 lands on 0x0A
    assert_eq!(i16_le(&r, 0x0A), Some(-12000));
    assert_eq!(i16_le(&r, 0x0C), Some(12001));
    assert_eq!(i16_le(&r, 0x18), Some(-1));
    assert_eq!(u32_le(&r, 0x1E), Some(0xDEAD_BEEF));
    assert_eq!(i16_le(&r, 0x2C), Some(i16::MIN));
}

#[test]
fn reads_never_leave_the_received_slice() {
    // T004 shape: a large backing buffer full of stale bytes, but only the
    // first `rx_len` bytes were actually received this time.
    let mut backing = [0xAAu8; 100];
    let r = report_45();
    backing[..r.len()].copy_from_slice(&r);
    let rx_len = 0x20; // short report: IMU tail not received
    let received = &backing[..rx_len];

    assert_eq!(u32_le(received, 0x02), Some(0x0020_0001));
    assert_eq!(u32_le(received, 0x1E), None); // straddles rx_len
    assert_eq!(i16_le(received, 0x2C), None); // exists in backing, not received
}
