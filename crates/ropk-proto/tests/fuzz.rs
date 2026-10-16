// T011: property-based fuzzing of the whole decode path on stable Rust.
// proptest generates thousands of inputs per property and, on failure, shrinks
// to a minimal case and saves its seed under proptest-regressions/ -- commit
// that file so the case is replayed forever. Also copy the shrunk payload into
// tests/fixtures/bad/ (or good/) with a comment saying what it caught.
//
// Seeds the M1 exit criteria name explicitly: every cut point, and
// stale-filled backing buffers.

mod common;

use common::{f1, tlv};
use proptest::prelude::*;
use proptest::sample::Index;
use ropk_proto::frame::{MAX_PAYLOAD_LEN, REPLY_STATUS, Reply, parse_reply};
use ropk_proto::report::{INPUT_FULL_LEN, INPUT_MIN_LEN, Report};
use ropk_proto::seq::{Freshness, SeqTracker};
use ropk_proto::tlv::{TAG_BLOB, TAG_REPORT, TAG_STATUS};

// ---- invariants that must hold for ANY input ----

fn check_invariants(p: &[u8]) -> Result<(), TestCaseError> {
    let reply = match parse_reply(p) {
        Ok(r) => r,
        Err(_) => {
            // the only ways the envelope can fail
            prop_assert!(p.is_empty() || p.len() > MAX_PAYLOAD_LEN || p[0] == REPLY_STATUS);
            return Ok(());
        }
    };
    let _ = reply.is_link_alive();
    if let Reply::Input(frame) = reply {
        let range = p.as_ptr_range();
        // each Ok TLV consumes >= 3 bytes, plus at most one trailing error
        let bound = p.len() / 3 + 2;
        for (n, item) in frame.reports().enumerate() {
            prop_assert!(n < bound, "walk did not terminate in {bound} steps");
            if let Ok(Report::Input(r)) = item {
                let rr = r.raw.as_ptr_range();
                prop_assert!(range.start <= rr.start && rr.end <= range.end);
                prop_assert!(r.raw.len() >= INPUT_MIN_LEN);
                // T007: IMU only ever comes from a complete tail
                prop_assert_eq!(r.input.imu.is_some(), r.raw.len() >= INPUT_FULL_LEN);
            }
        }
    }
    Ok(())
}

// ---- valid frame generator ----

#[derive(Clone, Debug)]
enum Rec {
    Input {
        id: u8,
        seq: u8,
        buttons: u32,
        len: usize,
        fill: u8,
    },
    Battery {
        charge: u8,
        pct: u8,
    },
    StatusField([u8; 4]),
    Blob(Vec<u8>),
}

fn rec() -> impl Strategy<Value = Rec> {
    prop_oneof![
        (
            any::<u8>(),
            any::<u32>(),
            INPUT_MIN_LEN..=INPUT_FULL_LEN,
            any::<u8>()
        )
            .prop_map(|(seq, buttons, len, fill)| Rec::Input {
                id: 0x45,
                seq,
                buttons,
                len,
                fill
            }),
        (any::<u8>(), any::<u32>(), any::<u8>()).prop_map(|(seq, buttons, fill)| Rec::Input {
            id: 0x42,
            seq,
            buttons,
            len: 54,
            fill
        }),
        (any::<u8>(), any::<u8>()).prop_map(|(charge, pct)| Rec::Battery { charge, pct }),
        any::<[u8; 4]>().prop_map(Rec::StatusField),
        prop::collection::vec(any::<u8>(), 1..8).prop_map(Rec::Blob),
    ]
}

fn encode(r: &Rec) -> Vec<u8> {
    match r {
        Rec::Input {
            id,
            seq,
            buttons,
            len,
            fill,
        } => {
            let mut v = vec![*fill; *len];
            v[0] = *id;
            v[1] = *seq;
            v[2..6].copy_from_slice(&buttons.to_le_bytes());
            tlv(TAG_REPORT, &v)
        }
        Rec::Battery { charge, pct } => tlv(TAG_REPORT, &[0x43, *charge, *pct]),
        Rec::StatusField(b) => tlv(TAG_STATUS, b),
        Rec::Blob(b) => tlv(TAG_BLOB, b),
    }
}

/// A valid F1 that fits in MAX_PAYLOAD_LEN, plus its records.
fn valid_f1() -> impl Strategy<Value = (Vec<u8>, Vec<Rec>)> {
    prop::collection::vec(rec(), 0..5).prop_map(|recs| {
        // keep the longest prefix that fits
        let mut kept = Vec::new();
        let mut len = 1;
        for r in recs {
            let n = encode(&r).len();
            if len + n > MAX_PAYLOAD_LEN {
                break;
            }
            len += n;
            kept.push(r);
        }
        let bytes = f1(&kept.iter().map(encode).collect::<Vec<_>>());
        (bytes, kept)
    })
}

/// What reports() should yield for these records (non-report tags skipped).
#[derive(Debug, PartialEq)]
enum Want {
    Input {
        id: u8,
        seq: u8,
        buttons: u32,
        len: usize,
    },
    Battery {
        charge: u8,
        pct: u8,
    },
}

fn expected(recs: &[Rec]) -> Vec<Want> {
    recs.iter()
        .filter_map(|r| match *r {
            Rec::Input {
                id,
                seq,
                buttons,
                len,
                ..
            } => Some(Want::Input {
                id,
                seq,
                buttons,
                len,
            }),
            Rec::Battery { charge, pct } => Some(Want::Battery { charge, pct }),
            _ => None,
        })
        .collect()
}

fn got(p: &[u8]) -> Vec<Want> {
    let Ok(Reply::Input(f)) = parse_reply(p) else {
        return vec![];
    };
    f.reports()
        .filter_map(|r| match r {
            Ok(Report::Input(i)) => Some(Want::Input {
                id: i.id,
                seq: i.seq,
                buttons: i.input.buttons.bits(),
                len: i.raw.len(),
            }),
            Ok(Report::Status(s)) => Some(Want::Battery {
                charge: s.charge_state,
                pct: s.battery_pct,
            }),
            _ => None,
        })
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 2000, ..ProptestConfig::default() })]

    // pure noise, every length up to just past the limit
    #[test]
    fn arbitrary_payload(p in prop::collection::vec(any::<u8>(), 0..=MAX_PAYLOAD_LEN + 4)) {
        check_invariants(&p)?;
    }

    // noise behind an F1 byte, so the TLV/report walk actually runs
    #[test]
    fn arbitrary_f1_body(body in prop::collection::vec(any::<u8>(), 0..MAX_PAYLOAD_LEN)) {
        let mut p = vec![0xF1];
        p.extend(body);
        check_invariants(&p)?;
    }

    // well-formed frames decode to exactly what was encoded, in order
    #[test]
    fn valid_frames_roundtrip((p, recs) in valid_f1()) {
        prop_assert!(p.len() <= MAX_PAYLOAD_LEN);
        let Ok(Reply::Input(f)) = parse_reply(&p) else { panic!("not F1") };
        prop_assert!(f.reports().all(|r| r.is_ok()), "valid frame produced an error");
        prop_assert_eq!(got(&p), expected(&recs));
    }

    // one corrupted byte anywhere: no panic, invariants hold
    #[test]
    fn one_corrupt_byte((mut p, _) in valid_f1(), at: Index, val: u8) {
        let i = at.index(p.len());
        p[i] = val;
        check_invariants(&p)?;
    }

    // T003 + T004 together: cut a valid frame anywhere inside a buffer full of
    // stale bytes. Only the received prefix may ever be decoded.
    #[test]
    fn cut_inside_stale_buffer((p, _) in valid_f1(), cut: Index, stale: u8) {
        let full = got(&p);
        let cut = cut.index(p.len() + 1);
        let mut backing = [stale; 100];
        backing[..p.len()].copy_from_slice(&p);
        let received = &backing[..cut];

        check_invariants(received)?;
        let part = got(received);
        // whatever decodes is a prefix of the full frame's reports
        prop_assert!(part.len() <= full.len());
        prop_assert_eq!(&part[..], &full[..part.len()]);
    }

    // freshness: one Fresh per change of value, missed counts the gap
    #[test]
    fn seq_tracker_counts(seqs in prop::collection::vec(any::<u8>(), 0..300)) {
        let mut t = SeqTracker::new();
        let mut prev: Option<u8> = None;
        for &s in &seqs {
            let f = t.observe(s);
            match prev {
                None => prop_assert_eq!(f, Freshness::Fresh { missed: 0 }),
                Some(p) if p == s => prop_assert_eq!(f, Freshness::Repeat),
                Some(p) => prop_assert_eq!(
                    f,
                    Freshness::Fresh { missed: s.wrapping_sub(p).wrapping_sub(1) }
                ),
            }
            prev = Some(s);
        }
        prop_assert_eq!(t.last(), seqs.last().copied());
    }
}
