// Contract for ropk_proto::input -- the decoded controller value types 
// Button bits are tested in sc_controller_buttons.rs.

use ropk_proto::input::{
    ControllerInput, Imu, LINK_TIMEOUT_MS, Sample, Sticks, Trackpad, Triggers, trigger_u8,
};
use ropk_proto::sc_controller_buttons::{Button, Buttons};

// ---- trigger_u8 ----

// Full pull reads ~0x8000, so the scale is >>7 with saturation, not >>8
#[test]
fn trigger_u8_scaling_and_saturation() {
    assert_eq!(trigger_u8(0x0000), 0);
    assert_eq!(trigger_u8(0x007F), 0);
    assert_eq!(trigger_u8(0x0080), 1);
    assert_eq!(trigger_u8(0x4000), 128);
    assert_eq!(trigger_u8(0x7F7F), 254);
    assert_eq!(trigger_u8(0x7F80), 255);
    assert_eq!(trigger_u8(0x7FFF), 255);
    assert_eq!(trigger_u8(0x8000), 255); // >>7 = 256, must clamp
    assert_eq!(trigger_u8(0xFFFF), 255);
}

#[test]
fn trigger_u8_is_monotonic() {
    let mut prev = 0;
    for raw in (0..=u16::MAX).step_by(61) {
        let v = trigger_u8(raw);
        assert!(v >= prev, "dropped at {raw:#06x}");
        prev = v;
    }
}

// ---- value structs ----

fn busy_input() -> ControllerInput {
    ControllerInput {
        buttons: Button::A | Button::RTrackpadTouch | Button::Lb,
        triggers: Triggers {
            l: 0x1234,
            r: 0x8000,
        },
        sticks: Sticks {
            lx: -12000,
            ly: 12001,
            rx: i16::MIN,
            ry: i16::MAX,
        },
        l_trackpad: Trackpad {
            x: 1,
            y: -2,
            press: 3,
        },
        r_trackpad: Trackpad {
            x: -4,
            y: 5,
            press: -6,
        },
        imu: Some(Imu {
            timestamp: 0xDEAD_BEEF,
            accel: [1, -1, 4096],
            gyro: [i16::MIN, 0, i16::MAX],
        }),
    }
}

#[test]
fn value_types_are_copy_and_comparable() {
    let a = busy_input();
    let b = a; // Copy: report.rs hands these out by value
    assert_eq!(a, b);
    let mut c = a;
    c.sticks.lx = 0;
    assert_ne!(a, c);
}

#[test]
fn neutral_is_all_zero_with_no_imu() {
    let n = ControllerInput::NEUTRAL;
    assert_eq!(n, ControllerInput::default());
    assert_eq!(n.buttons, Buttons::EMPTY);
    assert_eq!(n.triggers, Triggers { l: 0, r: 0 });
    assert_eq!(
        n.sticks,
        Sticks {
            lx: 0,
            ly: 0,
            rx: 0,
            ry: 0
        }
    );
    assert_eq!(
        n.l_trackpad,
        Trackpad {
            x: 0,
            y: 0,
            press: 0
        }
    );
    assert_eq!(
        n.r_trackpad,
        Trackpad {
            x: 0,
            y: 0,
            press: 0
        }
    );
    // T007: missing IMU is "no motion data", never fabricated zero motion
    assert_eq!(n.imu, None);
}

#[test]
fn is_neutral_ignores_imu() {
    // a resting controller still streams gravity on the accelerometer
    let mut resting = ControllerInput::NEUTRAL;
    resting.imu = Some(Imu {
        timestamp: 1,
        accel: [0, 0, 4096],
        gyro: [0; 3],
    });
    assert!(resting.is_neutral());
    assert!(!busy_input().is_neutral());

    let mut one_bit = ControllerInput::NEUTRAL;
    one_bit.buttons = Button::RTrackpadTouch.into();
    assert!(!one_bit.is_neutral());

    let mut one_axis = ControllerInput::NEUTRAL;
    one_axis.r_trackpad.y = -1;
    assert!(!one_axis.is_neutral());
}

// ---- Sample: link freshness (M5) ----
// A slot is alive while its last reply is < 300 ms old (rf_link.cpp:161).
// Past that, consumers must see NEUTRAL so a key held at the moment the
// controller died gets released instead of sticking on the desktop.

#[test]
fn link_timeout_matches_c() {
    assert_eq!(LINK_TIMEOUT_MS, 300);
}

#[test]
fn fresh_sample_passes_input_through() {
    let s = Sample {
        input: busy_input(),
        received_ms: 1_000,
    };
    assert!(s.is_fresh(1_000));
    assert_eq!(s.current(1_000), busy_input());
    assert_eq!(s.current(1_000 + LINK_TIMEOUT_MS - 1), busy_input());
}

#[test]
fn stale_sample_is_neutral_at_exact_boundary() {
    let s = Sample {
        input: busy_input(),
        received_ms: 1_000,
    };
    assert!(!s.is_fresh(1_000 + LINK_TIMEOUT_MS));
    assert_eq!(s.current(1_000 + LINK_TIMEOUT_MS), ControllerInput::NEUTRAL);
    assert_eq!(s.current(u64::MAX), ControllerInput::NEUTRAL);
}

#[test]
fn stale_sample_releases_held_buttons() {
    let mut held = ControllerInput::NEUTRAL;
    held.buttons = Button::A | Button::RT;
    let s = Sample {
        input: held,
        received_ms: 0,
    };
    assert!(s.current(10_000).buttons.is_empty());
}

#[test]
fn age_saturates_when_clock_is_behind_sample() {
    // timestamps from two sources can disagree by a tick; must not underflow
    let s = Sample {
        input: busy_input(),
        received_ms: 5_000,
    };
    assert_eq!(s.age_ms(4_999), 0);
    assert!(s.is_fresh(4_999));
    assert_eq!(s.age_ms(5_250), 250);
}

#[test]
fn default_sample_is_never_fresh_input() {
    // before the first reply arrives a slot must read as neutral, even if
    // the clock is still near zero
    let s = Sample::default();
    assert_eq!(s.current(0), ControllerInput::NEUTRAL);
    assert_eq!(s.current(LINK_TIMEOUT_MS * 10), ControllerInput::NEUTRAL);
}
