// Contract for ropk_proto::frame -- one radio reply payload ([rtype][body..],
// i.e. ScRadioData.payload) -> Reply, and an F1 body -> its reports.
// Source: rf_link.cpp:299-430, PROTOCOL.md §7.3. Covers T004, T006, T008.

use ropk_proto::frame::*;
use ropk_proto::report::{INPUT_MIN_LEN, Report, ReportError};
use ropk_proto::tlv::{TAG_BLOB, TAG_REPORT, TAG_STATUS, TlvError};

// ---- fixtures ----

fn tlv(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut v = vec![value.len() as u8, tag];
    v.extend_from_slice(value);
    v
}

fn f1(records: &[Vec<u8>]) -> Vec<u8> {
    let mut v = vec![REPLY_INPUT];
    for r in records {
        v.extend_from_slice(r);
    }
    v
}

// Minimum-length input report (no IMU) so two fit in one 96-byte F1.
fn input_report(id: u8, seq: u8, buttons: u32) -> Vec<u8> {
    let mut r = vec![0u8; INPUT_MIN_LEN];
    r[0] = id;
    r[1] = seq;
    r[2..6].copy_from_slice(&buttons.to_le_bytes());
    r
}

fn seqs(frame: InputFrame) -> Vec<Result<u8, ReportItemError>> {
    frame
        .reports()
        .map(|r| {
            r.map(|rep| match rep {
                Report::Input(i) => i.seq,
                other => panic!("unexpected {other:?}"),
            })
        })
        .collect()
}

fn input_frame(payload: &[u8]) -> InputFrame<'_> {
    match parse_reply(payload) {
        Ok(Reply::Input(f)) => f,
        other => panic!("expected Input, got {other:?}"),
    }
}

// ---- constants ----

#[test]
fn constants_match_c() {
    assert_eq!(MAX_PAYLOAD_LEN, 96);
    assert_eq!(REPLY_INPUT, 0xF1);
    assert_eq!(REPLY_DISCONNECT, 0xF2);
    assert_eq!(REPLY_STATUS, 0xF3);
    assert_eq!(REPLY_OURS_MIN, 0xF0);
    // C reads rfrx[6]; payload drops the 2-byte [LEN][S1] header
    assert_eq!(OFF_F3_VERSION, 4);
    assert_eq!(F3_MIN_LEN, OFF_F3_VERSION + 1);
}

// ---- envelope: length ----

#[test]
fn empty_payload_is_error() {
    assert_eq!(parse_reply(&[]), Err(FrameError::Empty));
}

#[test]
fn too_long_is_error() {
    let mut p = vec![0u8; MAX_PAYLOAD_LEN + 1];
    p[0] = REPLY_INPUT;
    assert_eq!(parse_reply(&p), Err(FrameError::TooLong { len: 97 }));
    // also for non-F1 types: length is checked before the type
    p[0] = 0xE1;
    assert_eq!(parse_reply(&p), Err(FrameError::TooLong { len: 97 }));
}

#[test]
fn exactly_max_len_is_accepted() {
    // [F1][len=93][tag][93 bytes] = 96
    let p = f1(&[tlv(TAG_BLOB, &[0x11; 93])]);
    assert_eq!(p.len(), MAX_PAYLOAD_LEN);
    assert!(matches!(parse_reply(&p), Ok(Reply::Input(_))));
}

// ---- T008: E-types are not ours ----

// every puck shares the "ibex" address; another puck's beacon/poll must never
// mark our controller alive (rf_link.cpp:313-317)
#[test]
fn t008_e_types_are_not_ours() {
    for p in [
        &[0xE1, 1, 2, 3][..],
        &[0xE3],
        &[0xE7, 0, 0],
        &[0x00],
        &[0x45],
        &[0xEF],
    ] {
        let reply = parse_reply(p).unwrap();
        assert_eq!(reply, Reply::NotOurs { rtype: p[0] });
        assert!(!reply.is_link_alive(), "{p:02x?}");
    }
}

#[test]
fn every_type_below_f0_is_not_ours() {
    for rtype in 0x00..REPLY_OURS_MIN {
        let p = [rtype, 0, 0, 0, 0, 0];
        let r = parse_reply(&p).unwrap();
        assert_eq!(r, Reply::NotOurs { rtype });
    }
}

// ---- F2 ----

#[test]
fn f2_is_disconnect() {
    assert_eq!(parse_reply(&[0xF2]), Ok(Reply::Disconnect));
    assert_eq!(parse_reply(&[0xF2, 9, 9, 9]), Ok(Reply::Disconnect));
    assert!(Reply::Disconnect.is_link_alive());
}

// ---- T008: F3 ----

#[test]
fn t008_f3_below_min_is_error_not_a_read() {
    for len in 1..F3_MIN_LEN {
        let mut p = vec![0u8; len];
        p[0] = REPLY_STATUS;
        assert_eq!(
            parse_reply(&p),
            Err(FrameError::StatusTooShort { len }),
            "len={len}"
        );
    }
}

#[test]
fn f3_reads_version_at_offset_4() {
    let p = [0xF3, 0xA1, 0xA2, 0xA3, 0x17, 0xA5];
    assert_eq!(
        parse_reply(&p),
        Ok(Reply::Status {
            version: 0x17,
            raw: &p
        })
    );
    let r = parse_reply(&p[..5]).unwrap();
    assert_eq!(
        r,
        Reply::Status {
            version: 0x17,
            raw: &p[..5]
        }
    );
    assert!(r.is_link_alive());
}

// ---- other F-types ----

#[test]
fn unknown_f_types_are_other_reply_and_alive() {
    for rtype in [0xF0, 0xF4, 0xF9, 0xFF] {
        let p = [rtype, 1, 2];
        let r = parse_reply(&p).unwrap();
        assert_eq!(r, Reply::OtherReply { rtype, raw: &p });
        // matches C (rtype >= 0xF0); see frame spec decision
        assert!(r.is_link_alive(), "{rtype:#04x}");
    }
}

#[test]
fn every_f_type_is_ours() {
    for rtype in REPLY_OURS_MIN..=0xFF {
        let p = [rtype, 0, 0, 0, 0, 0];
        let r = parse_reply(&p).unwrap();
        assert!(!matches!(r, Reply::NotOurs { .. }), "{rtype:#04x}");
        assert!(r.is_link_alive(), "{rtype:#04x}");
    }
}

// ---- F1: envelope ----

#[test]
fn f1_alone_is_input_with_no_reports() {
    let f = input_frame(&[REPLY_INPUT]);
    assert_eq!(f.reports().count(), 0);
    assert_eq!(f.tlvs().count(), 0);
    assert!(Reply::Input(f).is_link_alive());
}

#[test]
fn f1_body_starts_after_rtype() {
    // the TLV walk must start at payload[1] (C: rfrx[3]), not "byte 6"
    let p = f1(&[tlv(TAG_STATUS, &[1, 2, 3, 4])]);
    let f = input_frame(&p);
    let t = f.tlvs().next().unwrap().unwrap();
    assert_eq!((t.tag, t.value), (TAG_STATUS, &[1u8, 2, 3, 4][..]));
}

// ---- F1: reports ----

#[test]
fn single_input_report() {
    let p = f1(&[tlv(TAG_REPORT, &input_report(0x45, 7, 0x1))]);
    let f = input_frame(&p);
    let items: Vec<_> = f.reports().collect();
    assert_eq!(items.len(), 1);
    let Ok(Report::Input(i)) = items[0] else {
        panic!()
    };
    assert_eq!((i.id, i.seq, i.input.buttons.bits()), (0x45, 7, 0x1));
}

// one F1 can carry several inputs; C: "taking only [0] halves the rate"
#[test]
fn two_inputs_in_one_f1_both_yielded_in_order() {
    let p = f1(&[
        tlv(TAG_REPORT, &input_report(0x45, 7, 0)),
        tlv(TAG_REPORT, &input_report(0x45, 8, 0)),
    ]);
    assert!(p.len() <= MAX_PAYLOAD_LEN);
    assert_eq!(seqs(input_frame(&p)), [Ok(7), Ok(8)]);
}

#[test]
fn non_report_tags_are_skipped_silently() {
    let p = f1(&[
        tlv(TAG_STATUS, &[1, 2, 3, 4]),
        tlv(TAG_REPORT, &input_report(0x42, 3, 0)),
        tlv(TAG_BLOB, &[0xAB, 0xCD]),
        tlv(0x7F, &[0x00]),
        tlv(TAG_REPORT, &[0x43, 2, 77]),
    ]);
    let items: Vec<_> = input_frame(&p).reports().collect();
    assert_eq!(items.len(), 2);
    assert!(matches!(items[0], Ok(Report::Input(i)) if i.id == 0x42 && i.seq == 3));
    assert!(matches!(items[1], Ok(Report::Status(s)) if s.battery_pct == 77));
    // ...while tlvs() still sees all five
    assert_eq!(input_frame(&p).tlvs().count(), 5);
}

#[test]
fn unknown_report_ids_pass_through_as_unknown() {
    let p = f1(&[tlv(TAG_REPORT, &[0x99, 1, 2])]);
    let items: Vec<_> = input_frame(&p).reports().collect();
    assert!(matches!(items[..], [Ok(Report::Unknown { id: 0x99, .. })]));
}

// ---- T006: partial-frame policy ----

// a malformed REPORT doesn't hide the next one (C: the if fails, idx moves on)
#[test]
fn t006_bad_report_then_good_continues() {
    let p = f1(&[
        tlv(TAG_REPORT, &input_report(0x45, 1, 0)[..10]),
        tlv(TAG_REPORT, &input_report(0x45, 2, 0)),
    ]);
    assert_eq!(
        seqs(input_frame(&p)),
        [
            Err(ReportItemError::Report(ReportError::InputTooShort {
                id: 0x45,
                len: 10
            })),
            Ok(2)
        ]
    );
}

#[test]
fn empty_report_tlv_value_is_report_error() {
    // [len=0] is the TLV terminator, so build an empty-ish report via a 1-byte
    // value of an unparseable status instead
    let p = f1(&[
        tlv(TAG_REPORT, &[0x43]),
        tlv(TAG_REPORT, &input_report(0x45, 4, 0)),
    ]);
    assert_eq!(
        seqs(input_frame(&p)),
        [
            Err(ReportItemError::Report(ReportError::StatusTooShort {
                len: 1
            })),
            Ok(4)
        ]
    );
}

// a broken TLV STRUCTURE stops the walk: nothing after it can be trusted
#[test]
fn t006_bad_tlv_stops() {
    let mut p = f1(&[tlv(TAG_REPORT, &input_report(0x45, 1, 0))]);
    let bad_at = p.len() - 1; // offset within the F1 body
    p.extend_from_slice(&[40, TAG_REPORT, 0x45, 0x02]); // claims 40, has 2
    let items = seqs(input_frame(&p));
    assert_eq!(
        items,
        [
            Ok(1),
            Err(ReportItemError::Tlv(TlvError::ValueTruncated {
                at: bad_at,
                len: 40,
                remaining: 2
            }))
        ]
    );
}

#[test]
fn trailing_lone_byte_is_tlv_error() {
    let mut p = f1(&[tlv(TAG_REPORT, &input_report(0x45, 1, 0))]);
    p.push(0x05);
    let items = seqs(input_frame(&p));
    assert!(matches!(
        items[..],
        [
            Ok(1),
            Err(ReportItemError::Tlv(TlvError::HeaderTruncated { .. }))
        ]
    ));
}

#[test]
fn zero_len_terminator_ends_reports() {
    let mut p = f1(&[tlv(TAG_REPORT, &input_report(0x45, 1, 0))]);
    p.extend_from_slice(&[0, 0]);
    p.extend_from_slice(&tlv(TAG_REPORT, &input_report(0x45, 2, 0))[..10]);
    assert_eq!(seqs(input_frame(&p)), [Ok(1)]);
}

// ---- iterator behaviour ----

#[test]
fn reports_can_be_walked_twice() {
    let p = f1(&[
        tlv(TAG_REPORT, &input_report(0x45, 5, 0)),
        tlv(TAG_REPORT, &input_report(0x45, 6, 0)),
    ]);
    let f = input_frame(&p);
    assert_eq!(seqs(f), seqs(f));
    assert_eq!(seqs(f), [Ok(5), Ok(6)]);
}

#[test]
fn reports_stay_none_after_end() {
    let p = f1(&[tlv(TAG_REPORT, &input_report(0x45, 1, 0))]);
    let mut it = input_frame(&p).reports();
    assert!(it.next().is_some());
    for _ in 0..3 {
        assert!(it.next().is_none());
    }
}

#[test]
fn reports_stay_none_after_tlv_error() {
    let p = [REPLY_INPUT, 9, TAG_REPORT, 0x45];
    let mut it = input_frame(&p).reports();
    assert!(matches!(it.next(), Some(Err(ReportItemError::Tlv(_)))));
    for _ in 0..3 {
        assert!(it.next().is_none());
    }
}

#[test]
fn reports_is_fused() {
    fn assert_fused<I: core::iter::FusedIterator>(_: &I) {}
    assert_fused(&input_frame(&[REPLY_INPUT]).reports());
}

#[test]
fn report_raw_points_into_the_payload() {
    let p = f1(&[tlv(TAG_REPORT, &input_report(0x45, 1, 0))]);
    let Some(Ok(Report::Input(i))) = input_frame(&p).reports().next() else {
        panic!()
    };
    assert_eq!(i.raw.as_ptr(), p[3..].as_ptr());
    assert_eq!(i.raw.len(), INPUT_MIN_LEN);
}

// ---- T004: stale radio buffer ----

#[test]
fn t004_stale_backing_bytes_never_become_input() {
    const STALE: u8 = 0xAA;
    let mut backing = [STALE; 100];
    // the TLV header claims a full 46-byte report, but only 12 bytes arrived;
    // a C-style sizeof(rfrx) bound would decode the 0xAA tail as live input
    let mut pkt = vec![REPLY_INPUT, 46, TAG_REPORT];
    pkt.extend_from_slice(&input_report(0x45, 1, 0)[..12]);
    backing[..pkt.len()].copy_from_slice(&pkt);
    let received = &backing[..pkt.len()];

    let items: Vec<_> = input_frame(received).reports().collect();
    assert!(matches!(
        items[..],
        [Err(ReportItemError::Tlv(TlvError::ValueTruncated {
            len: 46,
            ..
        }))]
    ));
}

#[test]
fn t004_values_stay_inside_received_slice() {
    let mut backing = [0xAAu8; 100];
    let pkt = f1(&[
        tlv(TAG_REPORT, &input_report(0x45, 1, 0)),
        tlv(TAG_REPORT, &[0x43, 1, 2]),
    ]);
    backing[..pkt.len()].copy_from_slice(&pkt);
    let received = &backing[..pkt.len()];
    let range = received.as_ptr_range();
    for item in input_frame(received).reports() {
        let raw = match item.unwrap() {
            Report::Input(i) => i.raw,
            Report::Status(s) => s.raw,
            other => panic!("{other:?}"),
        };
        let r = raw.as_ptr_range();
        assert!(range.start <= r.start && r.end <= range.end);
        assert!(!raw.contains(&0xAA));
    }
}

// ---- T011-lite ----

#[test]
fn arbitrary_payloads_never_panic_and_terminate() {
    let mut seed: u32 = 0x0BAD_F00D;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed as u8
    };
    for len in 0..=MAX_PAYLOAD_LEN + 4 {
        for _ in 0..40 {
            let mut p: Vec<u8> = (0..len).map(|_| next()).collect();
            // bias half the cases towards F1 so the report walk gets exercised
            if let Some(b) = p.first_mut()
                && next() & 1 == 0
            {
                *b = REPLY_INPUT;
            }
            if let Ok(reply) = parse_reply(&p) {
                let _ = reply.is_link_alive();
                if let Reply::Input(f) = reply {
                    // each Ok TLV consumes >= 3 bytes
                    assert!(f.reports().count() <= len / 3 + 1);
                }
            }
        }
    }
}