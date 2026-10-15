// Contract for ropk_proto::seq -- per-slot input report freshness.
// The controller bumps report[0x01] once per new sample; a poll that lands
// before the next sample gets the same report again (same seq).
// C: rf_link.cpp:137, 450-458. Covers T009.

use ropk_proto::seq::{Freshness, SeqTracker};

const FRESH: Freshness = Freshness::Fresh { missed: 0 };

fn fresh(missed: u8) -> Freshness {
    Freshness::Fresh { missed }
}

// ---- first observation ----

// C bug: g_lastSeq starts at 0, so a first report with seq 0 was dropped as
// a repeat. An unseen tracker must accept anything.
#[test]
fn first_report_is_fresh_even_seq_0() {
    let mut t = SeqTracker::new();
    assert_eq!(t.observe(0), FRESH);
}

#[test]
fn first_report_is_fresh_for_any_value() {
    for seq in [0, 1, 127, 200, 255] {
        let mut t = SeqTracker::new();
        assert_eq!(t.observe(seq), FRESH, "seq={seq}");
    }
}

// ---- repeats ----

#[test]
fn same_seq_is_repeat() {
    let mut t = SeqTracker::new();
    assert_eq!(t.observe(5), FRESH);
    assert_eq!(t.observe(5), Freshness::Repeat);
    assert_eq!(t.observe(5), Freshness::Repeat);
}

#[test]
fn repeat_does_not_change_state() {
    let mut t = SeqTracker::new();
    t.observe(5);
    t.observe(5);
    // still measured from 5, not disturbed by the repeat
    assert_eq!(t.observe(6), FRESH);
    assert_eq!(t.last(), Some(6));
}

// ---- missed count ----

#[test]
fn next_seq_missed_nothing() {
    let mut t = SeqTracker::new();
    t.observe(5);
    assert_eq!(t.observe(6), FRESH);
}

#[test]
fn gap_counts_skipped_samples() {
    let mut t = SeqTracker::new();
    t.observe(7);
    assert_eq!(t.observe(10), fresh(2)); // 8 and 9 never seen
}

#[test]
fn wrap_255_to_0_is_fresh_missed_nothing() {
    let mut t = SeqTracker::new();
    t.observe(255);
    assert_eq!(t.observe(0), FRESH);
}

#[test]
fn wrap_with_gap() {
    let mut t = SeqTracker::new();
    t.observe(254);
    assert_eq!(t.observe(1), fresh(2)); // 255 and 0 never seen
}

// one byte can't tell "went backwards" from "lost 254": still fresh, and the
// big missed value is the diagnostic signal that something odd happened
#[test]
fn backwards_is_fresh_with_large_missed() {
    let mut t = SeqTracker::new();
    t.observe(10);
    assert_eq!(t.observe(9), fresh(254));
}

#[test]
fn missed_is_wrapping_distance_minus_one() {
    for last in [0u8, 1, 100, 254, 255] {
        for seq in [0u8, 1, 2, 128, 254, 255] {
            if seq == last {
                continue;
            }
            let mut t = SeqTracker::new();
            t.observe(last);
            let want = seq.wrapping_sub(last).wrapping_sub(1);
            assert_eq!(t.observe(seq), fresh(want), "{last} -> {seq}");
        }
    }
}

#[test]
fn full_cycle_is_all_fresh_missed_zero() {
    let mut t = SeqTracker::new();
    for seq in 0..=255u8 {
        assert_eq!(t.observe(seq), FRESH, "seq={seq}");
    }
    assert_eq!(t.observe(0), FRESH, "wrap after full cycle");
}

// ---- reset ----

// C never resets on reconnect: a rebooted controller whose first seq matched
// the stale value was dropped. reset() is the fix.
#[test]
fn reset_makes_next_report_fresh() {
    let mut t = SeqTracker::new();
    t.observe(5);
    t.reset();
    assert_eq!(t.observe(5), FRESH);
}

#[test]
fn reset_does_not_report_a_gap() {
    let mut t = SeqTracker::new();
    t.observe(5);
    t.reset();
    // a reconnect isn't "lost 94 samples"
    assert_eq!(t.observe(100), FRESH);
}

#[test]
fn reset_returns_to_new_state() {
    let mut t = SeqTracker::new();
    t.observe(42);
    t.reset();
    assert_eq!(t, SeqTracker::new());
}

// ---- last() ----

#[test]
fn last_tracks_latest_fresh_seq() {
    let mut t = SeqTracker::new();
    assert_eq!(t.last(), None);
    t.observe(5);
    assert_eq!(t.last(), Some(5));
    t.observe(5);
    assert_eq!(t.last(), Some(5));
    t.observe(9);
    assert_eq!(t.last(), Some(9));
    t.reset();
    assert_eq!(t.last(), None);
}

// ---- construction ----

#[test]
fn new_is_const_and_equals_default() {
    const T: SeqTracker = SeqTracker::new();
    // per-slot array initialised at compile time, as the firmware will do
    const SLOTS: [SeqTracker; 4] = [SeqTracker::new(); 4];
    assert_eq!(T, SeqTracker::default());
    assert!(SLOTS.iter().all(|s| s.last().is_none()));
}

#[test]
fn trackers_are_independent() {
    let mut slots = [SeqTracker::new(); 4];
    slots[0].observe(5);
    slots[1].observe(5);
    assert_eq!(slots[0].observe(5), Freshness::Repeat);
    // slot 2 never saw 5: a shared tracker would wrongly call this a repeat
    assert_eq!(slots[2].observe(5), FRESH);
    slots[1].reset();
    assert_eq!(slots[1].observe(5), FRESH);
    assert_eq!(slots[0].last(), Some(5));
}

#[test]
fn tracker_is_copy() {
    let mut a = SeqTracker::new();
    a.observe(3);
    let mut b = a;
    b.observe(4);
    assert_eq!(a.last(), Some(3));
    assert_eq!(b.last(), Some(4));
}

// ---- Freshness ----

#[test]
fn is_fresh_helper() {
    assert!(FRESH.is_fresh());
    assert!(fresh(200).is_fresh());
    assert!(!Freshness::Repeat.is_fresh());
}

#[test]
fn is_fresh_is_const() {
    // evaluated at compile time: fails the build, not the run, if not const
    const { assert!(Freshness::Fresh { missed: 1 }.is_fresh()) };
    const { assert!(!Freshness::Repeat.is_fresh()) };
}

// ---- realistic stream ----

// 4 ms polls against a slower sample rate: repeats interleave with fresh
// reports, and one dropped reply shows up as missed = 1
#[test]
fn poll_stream() {
    let mut t = SeqTracker::new();
    let stream = [10, 10, 11, 11, 11, 12, 14, 14, 15];
    let got: Vec<Freshness> = stream.iter().map(|&s| t.observe(s)).collect();
    use Freshness::Repeat as R;
    assert_eq!(got, [FRESH, R, FRESH, R, R, FRESH, fresh(1), R, FRESH]);
    let new_count = got.iter().filter(|f| f.is_fresh()).count();
    assert_eq!(new_count, 5);
}