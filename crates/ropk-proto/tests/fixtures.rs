// Runs every payload in tests/fixtures/ through the full decode path
// (parse_reply -> reports). Good fixtures must decode with zero errors;
// bad ones must error somewhere and never panic. Real captures from the
// board (defmt `Payload :` lines) and minimized fuzz failures go here too.
//
// Inspect any of them by hand with:
//   cargo xtask decode --file crates/ropk-proto/tests/fixtures/good

mod common;

use common::{Fixture, load};
use ropk_proto::frame::{Reply, parse_reply};
use ropk_proto::report::Report;
use ropk_proto::sc_controller_buttons::{Button, Buttons};

/// (input reports, any error)
fn run(f: &Fixture) -> (usize, bool) {
    let reply = match parse_reply(&f.bytes) {
        Ok(r) => r,
        Err(_) => return (0, true),
    };
    let Reply::Input(frame) = reply else {
        return (0, false);
    };
    let mut inputs = 0;
    let mut err = false;
    for item in frame.reports() {
        match item {
            Ok(Report::Input(r)) => {
                inputs += 1;
                // every decoded report must lie inside the received payload
                let range = f.bytes.as_ptr_range();
                let r = r.raw.as_ptr_range();
                assert!(
                    range.start <= r.start && r.end <= range.end,
                    "{}",
                    f.label()
                );
            }
            Ok(_) => {}
            Err(_) => err = true,
        }
    }
    (inputs, err)
}

#[test]
fn fixture_dirs_are_populated() {
    assert!(load("good").len() >= 10);
    assert!(load("bad").len() >= 5);
}

#[test]
fn good_fixtures_decode_without_errors() {
    for f in load("good") {
        let (inputs, err) = run(&f);
        assert!(!err, "{}: unexpected decode error", f.label());
        if let Some(want) = f.expect_inputs {
            assert_eq!(inputs, want, "{}: input report count", f.label());
        }
    }
}

#[test]
fn bad_fixtures_all_error_without_panicking() {
    for f in load("bad") {
        let (_, err) = run(&f);
        assert!(err, "{}: expected a decode error, got none", f.label());
    }
}

#[test]
fn not_ours_never_marks_link_alive() {
    let fx: Vec<_> = load("good")
        .into_iter()
        .filter(|f| f.file.ends_with("not_ours.hex"))
        .collect();
    assert!(!fx.is_empty());
    for f in fx {
        let r = parse_reply(&f.bytes).unwrap();
        assert!(matches!(r, Reply::NotOurs { .. }), "{}", f.label());
        assert!(!r.is_link_alive(), "{}", f.label());
    }
}

#[test]
fn control_replies_mark_link_alive() {
    for f in load("good")
        .into_iter()
        .filter(|f| f.file.ends_with("control.hex"))
    {
        assert!(
            parse_reply(&f.bytes).unwrap().is_link_alive(),
            "{}",
            f.label()
        );
    }
}

// Guard against a fixture being edited into something that still parses but
// no longer means what its comment says.
#[test]
fn f1_input_first_fixture_has_documented_values() {
    let f = load("good")
        .into_iter()
        .find(|f| f.file.ends_with("f1_input.hex"))
        .unwrap();
    let Ok(Reply::Input(frame)) = parse_reply(&f.bytes) else {
        panic!()
    };
    let Some(Ok(Report::Input(r))) = frame.reports().next() else {
        panic!()
    };
    assert_eq!((r.id, r.seq), (0x45, 0x5A));
    let want: Buttons = Button::A | Button::Start | Button::RTrackpadTouch | Button::RTrackpadClick;
    assert_eq!(r.input.buttons, want);
    assert_eq!((r.input.triggers.l, r.input.triggers.r), (0x1234, 0x8000));
    assert_eq!((r.input.sticks.lx, r.input.sticks.ry), (-1001, 1004));
    let imu = r.input.imu.unwrap();
    assert_eq!(imu.timestamp, 0xDEAD_BEEF);
    assert_eq!(imu.accel, [101, -102, 4096]);
    assert_eq!(imu.gyro, [-201, 202, -203]);
}

#[test]
fn hex_parser_accepts_defmt_lines() {
    assert_eq!(
        common::parse_hex("0.512 INFO  Payload  : [f1, 2e, 06]").unwrap(),
        [0xF1, 0x2E, 0x06]
    );
    assert_eq!(common::parse_hex("f12e06").unwrap(), [0xF1, 0x2E, 0x06]);
    assert!(common::parse_hex("f1 2").is_err());
}
