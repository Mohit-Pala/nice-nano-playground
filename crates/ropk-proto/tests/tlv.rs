// Contract for ropk_proto::tlv -- the F1 container walker.
// Input is the F1 body: ScRadioData.payload[1..] (C: rfrx[3..], rf_link.cpp:426).
// Record = [len][tag][value; len]. Covers T003-T006 from the test catalogue.

use ropk_proto::tlv::{HEADER_LEN, TAG_BLOB, TAG_REPORT, TAG_STATUS, Tlv, TlvError, TlvIter};

fn ok(tag: u8, value: &[u8]) -> Option<Result<Tlv<'_>, TlvError>> {
    Some(Ok(Tlv { tag, value }))
}

// Three records: a status field, a short report and an empty-ish blob.
const FRAME: [u8; 15] = [
    4, TAG_STATUS, 0x01, 0x02, 0x03, 0x04, // [0..6]
    3, TAG_REPORT, 0x45, 0x07, 0x01, // [6..11]
    2, TAG_BLOB, 0xAB, 0xCD, // [11..15]
];

// ---- constants ----

#[test]
fn constants_match_protocol() {
    assert_eq!(TAG_STATUS, 0x02);
    assert_eq!(TAG_BLOB, 0x04);
    assert_eq!(TAG_REPORT, 0x06);
    assert_eq!(HEADER_LEN, 2);
}

// ---- happy paths ----

#[test]
fn empty_buffer_ends_immediately() {
    let mut it = TlvIter::new(&[]);
    assert_eq!(it.next(), None);
}

#[test]
fn single_record() {
    let buf = [3, TAG_REPORT, 0xA, 0xB, 0xC];
    let mut it = TlvIter::new(&buf);
    assert_eq!(it.next(), ok(TAG_REPORT, &[0xA, 0xB, 0xC]));
    assert_eq!(it.next(), None);
}

#[test]
fn multiple_records_in_order() {
    let mut it = TlvIter::new(&FRAME);
    assert_eq!(it.next(), ok(TAG_STATUS, &[0x01, 0x02, 0x03, 0x04]));
    assert_eq!(it.next(), ok(TAG_REPORT, &[0x45, 0x07, 0x01]));
    assert_eq!(it.next(), ok(TAG_BLOB, &[0xAB, 0xCD]));
    assert_eq!(it.next(), None);
}

#[test]
fn value_is_exactly_len_bytes() {
    for tlv in TlvIter::new(&FRAME) {
        let tlv = tlv.unwrap();
        assert!(!tlv.value.is_empty());
    }
    let lens: Vec<usize> = TlvIter::new(&FRAME)
        .map(|t| t.unwrap().value.len())
        .collect();
    assert_eq!(lens, [4, 3, 2]);
}

#[test]
fn unknown_tag_is_yielded_not_dropped() {
    // filtering by tag is report.rs's job; the walker stays neutral
    let buf = [1, 0x7F, 0x99];
    let mut it = TlvIter::new(&buf);
    assert_eq!(it.next(), ok(0x7F, &[0x99]));
    assert_eq!(it.next(), None);
}

#[test]
fn max_len_record_that_fits() {
    let mut buf = [0u8; 2 + 255];
    buf[0] = 255;
    buf[1] = TAG_BLOB;
    let mut it = TlvIter::new(&buf);
    let t = it.next().unwrap().unwrap();
    assert_eq!(t.tag, TAG_BLOB);
    assert_eq!(t.value.len(), 255);
    assert_eq!(it.next(), None);
}

// ---- terminator ----

// len == 0 stops the walk (rf_link.cpp:432); anything after is ignored
#[test]
fn zero_len_terminates() {
    let buf = [2, TAG_STATUS, 0x11, 0x22, 0, 0xDE, 0xAD, 0xBE, 0xEF];
    let mut it = TlvIter::new(&buf);
    assert_eq!(it.next(), ok(TAG_STATUS, &[0x11, 0x22]));
    assert_eq!(it.next(), None);
}

#[test]
fn zero_len_first_yields_nothing() {
    let buf = [0, TAG_REPORT, 0x45];
    assert_eq!(TlvIter::new(&buf).count(), 0);
}

// ---- truncation errors ----

#[test]
fn lone_trailing_byte_is_header_truncated() {
    let buf = [3, TAG_REPORT, 0xA, 0xB, 0xC, 0x09];
    let mut it = TlvIter::new(&buf);
    assert_eq!(it.next(), ok(TAG_REPORT, &[0xA, 0xB, 0xC]));
    assert_eq!(it.next(), Some(Err(TlvError::HeaderTruncated { at: 5 })));
    assert_eq!(it.next(), None);
}

#[test]
fn single_byte_buffer_is_header_truncated() {
    let mut it = TlvIter::new(&[7]);
    assert_eq!(it.next(), Some(Err(TlvError::HeaderTruncated { at: 0 })));
    assert_eq!(it.next(), None);
}

#[test]
fn value_overrun_is_value_truncated() {
    let buf = [5, TAG_REPORT, 0xA, 0xB];
    let mut it = TlvIter::new(&buf);
    assert_eq!(
        it.next(),
        Some(Err(TlvError::ValueTruncated {
            at: 0,
            len: 5,
            remaining: 2
        }))
    );
    assert_eq!(it.next(), None);
}

#[test]
fn header_only_is_value_truncated() {
    let buf = [1, TAG_REPORT];
    let mut it = TlvIter::new(&buf);
    assert_eq!(
        it.next(),
        Some(Err(TlvError::ValueTruncated {
            at: 0,
            len: 1,
            remaining: 0
        }))
    );
}

// T005: len = 0xFF in a short packet must error, not scan or wrap
#[test]
fn max_len_in_short_buffer_is_value_truncated() {
    let mut buf = [0u8; 10];
    buf[0] = 0xFF;
    buf[1] = TAG_REPORT;
    let mut it = TlvIter::new(&buf);
    assert_eq!(
        it.next(),
        Some(Err(TlvError::ValueTruncated {
            at: 0,
            len: 0xFF,
            remaining: 8
        }))
    );
    assert_eq!(it.next(), None);
}

#[test]
fn error_offset_points_at_the_bad_record() {
    // two good records, then a bad one starting at offset 11
    let mut buf = FRAME[..11].to_vec();
    buf.extend_from_slice(&[9, TAG_BLOB, 0x01]);
    let items: Vec<_> = TlvIter::new(&buf).collect();
    assert_eq!(items.len(), 3);
    assert_eq!(
        items[2],
        Err(TlvError::ValueTruncated {
            at: 11,
            len: 9,
            remaining: 1
        })
    );
}

// ---- T006: partial-frame policy ----

// records before a bad one are delivered; nothing after it is
#[test]
fn good_bad_good_stops_at_bad() {
    let buf = [
        1, TAG_STATUS, 0x01, // good
        4, TAG_REPORT, 0x45, // bad: claims 4, has 1... and swallows the rest
    ];
    let mut it = TlvIter::new(&buf);
    assert_eq!(it.next(), ok(TAG_STATUS, &[0x01]));
    assert!(matches!(
        it.next(),
        Some(Err(TlvError::ValueTruncated { at: 3, .. }))
    ));
    assert_eq!(it.next(), None);
}

#[test]
fn nothing_after_an_error_is_ever_yielded() {
    // a lone trailing byte after the error position can't resurrect the walk
    let buf = [1, TAG_STATUS, 0x01, 0x09];
    let items: Vec<_> = TlvIter::new(&buf).collect();
    assert_eq!(
        items,
        [
            Ok(Tlv {
                tag: TAG_STATUS,
                value: &[0x01]
            }),
            Err(TlvError::HeaderTruncated { at: 3 })
        ]
    );
}

// ---- fused ----

#[test]
fn stays_none_after_clean_end() {
    let mut it = TlvIter::new(&FRAME);
    while it.next().is_some() {}
    for _ in 0..3 {
        assert_eq!(it.next(), None);
    }
}

#[test]
fn stays_none_after_error() {
    let mut it = TlvIter::new(&[5, TAG_REPORT, 0xA]);
    assert!(matches!(it.next(), Some(Err(_))));
    for _ in 0..3 {
        assert_eq!(it.next(), None);
    }
}

#[test]
fn stays_none_after_terminator() {
    let mut it = TlvIter::new(&[0, 0, 1, TAG_BLOB, 0x01]);
    assert_eq!(it.next(), None);
    for _ in 0..3 {
        assert_eq!(it.next(), None);
    }
}

#[test]
fn is_fused_iterator() {
    fn assert_fused<I: core::iter::FusedIterator>(_: &I) {}
    assert_fused(&TlvIter::new(&FRAME));
}

// ---- T003: every cut point ----

// Truncating a valid frame anywhere must never panic, never yield a value
// shorter than its header claims, and the Ok items must be an exact prefix
// of the full frame's records.
#[test]
fn every_cut_point_is_safe() {
    let full: Vec<Tlv> = TlvIter::new(&FRAME).map(Result::unwrap).collect();
    for cut in 0..=FRAME.len() {
        let items: Vec<_> = TlvIter::new(&FRAME[..cut]).collect();
        let oks: Vec<Tlv> = items.iter().filter_map(|r| r.ok()).collect();

        assert_eq!(oks[..], full[..oks.len()], "cut={cut}");
        // at most one error, and only as the final item
        let errs = items.iter().filter(|r| r.is_err()).count();
        assert!(errs <= 1, "cut={cut}");
        if errs == 1 {
            assert!(items.last().unwrap().is_err(), "cut={cut}");
        }
        // a cut exactly on a record boundary is a clean end, not an error
        let on_boundary = [0, 6, 11, 15].contains(&cut);
        assert_eq!(errs == 0, on_boundary, "cut={cut}");
    }
}

// ---- T004: stale backing buffer ----

// The radio RAM buffer is reused; bytes past this packet's length are left
// over from an older packet. Only the received slice may ever be read.
#[test]
fn never_reads_past_received_slice() {
    const STALE: u8 = 0xAA;
    let mut backing = [STALE; 100];
    // header claims 6 bytes, but only 2 arrived this time
    let pkt = [6, TAG_REPORT, 0x45, 0x01];
    backing[..pkt.len()].copy_from_slice(&pkt);
    let received = &backing[..pkt.len()];

    let items: Vec<_> = TlvIter::new(received).collect();
    assert_eq!(
        items,
        [Err(TlvError::ValueTruncated {
            at: 0,
            len: 6,
            remaining: 2
        })]
    );
}

#[test]
fn yielded_values_stay_inside_received_slice() {
    let mut backing = [0xAAu8; 100];
    backing[..FRAME.len()].copy_from_slice(&FRAME);
    let received = &backing[..FRAME.len()];
    let range = received.as_ptr_range();
    for tlv in TlvIter::new(received) {
        let v = tlv.unwrap().value;
        let vr = v.as_ptr_range();
        assert!(range.start <= vr.start && vr.end <= range.end);
        assert!(!v.contains(&0xAA));
    }
}

// ---- T005 / T011-lite: always terminates ----

// Arbitrary bytes: the walk must finish within len/3 + 2 steps (each Ok
// consumes >= 3 bytes; at most one trailing error or terminator).
#[test]
fn arbitrary_bytes_always_terminate() {
    let mut seed: u32 = 0x1234_5678;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed as u8
    };
    for len in 0..=96 {
        for _ in 0..50 {
            let buf: Vec<u8> = (0..len).map(|_| next()).collect();
            let bound = len / 3 + 2;
            let mut steps = 0;
            for item in TlvIter::new(&buf) {
                steps += 1;
                assert!(steps <= bound, "len={len} buf={buf:02x?}");
                if let Ok(t) = item {
                    assert!(!t.value.is_empty());
                }
            }
        }
    }
}

// ---- lifetimes ----

// Tlv borrows the radio buffer, not the iterator: values must outlive it.
#[test]
fn values_outlive_the_iterator() {
    let values: Vec<&[u8]> = {
        let it = TlvIter::new(&FRAME);
        it.map(|t| t.unwrap().value).collect()
    };
    assert_eq!(values, [&FRAME[2..6], &FRAME[8..11], &FRAME[13..15]]);
}
