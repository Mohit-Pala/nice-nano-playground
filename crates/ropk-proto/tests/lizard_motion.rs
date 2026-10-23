// Contract for ropk_proto::lizard::motion -- right pad -> mouse (with glide),
// left pad -> scroll wheel.
// C: mode_lizard.cpp rfLizard() "right pad -> mouse motion with glide" and
//    "left pad -> scroll wheel", rfLizardRelease() (state reset), config.cpp
//    g_mDiv = 64 / g_mFric = 94, webusb_config.cpp clamps (div >= 4, fric <= 99).
//
// This is float state, so the tests pin the implementation to an exact port of
// the C arithmetic (`RefMouse` / `RefScroll` at the bottom), same f32 operations
// in the same order. Hand-worked cases come first, then proptest compares
// random multi-controller sequences bit-for-bit against the reference.

use proptest::prelude::*;
use ropk_proto::bond::NUM_SLOTS;
use ropk_proto::input::ControllerInput;
use ropk_proto::lizard::motion::{MotionConfig, PadMouse, PadScroll, WHEEL_MAX};
use ropk_proto::sc_controller_buttons::{Button, Buttons};

// ---- input builders ----

/// one slot's right pad: touching at (x, y), or lifted
fn rpad(touch: bool, x: i16, y: i16) -> ControllerInput {
    let mut i = ControllerInput::NEUTRAL;
    if touch {
        i.buttons = Buttons::EMPTY.with(Button::RTrackpadTouch);
    }
    i.r_trackpad.x = x;
    i.r_trackpad.y = y;
    i
}

/// one slot's left pad
fn lpad(touch: bool, y: i16) -> ControllerInput {
    let mut i = ControllerInput::NEUTRAL;
    if touch {
        i.buttons = Buttons::EMPTY.with(Button::LTrackpadTouch);
    }
    i.l_trackpad.y = y;
    i
}

fn one(i: &ControllerInput) -> [Option<&ControllerInput>; NUM_SLOTS] {
    [Some(i), None, None, None]
}

const CFG: MotionConfig = MotionConfig::DEFAULT;
const NO_GLIDE: MotionConfig = MotionConfig::new(64, 0);

// drive slot 0 of a PadMouse through a list of frames, return every output
fn mouse_run(cfg: &MotionConfig, frames: &[ControllerInput]) -> Vec<(i8, i8)> {
    let mut m = PadMouse::NEW;
    frames.iter().map(|f| m.step(&one(f), cfg)).collect()
}

fn scroll_run(cfg: &MotionConfig, frames: &[ControllerInput]) -> Vec<i8> {
    let mut s = PadScroll::NEW;
    frames.iter().map(|f| s.step(&one(f), cfg)).collect()
}

// ---- config ----

#[test]
fn default_config_is_c_default() {
    assert_eq!(MotionConfig::DEFAULT.div, 64);
    assert_eq!(MotionConfig::DEFAULT.friction_pct, 94);
    assert_eq!(MotionConfig::default(), MotionConfig::DEFAULT);
}

// webusb_config.cpp: g_mDiv = v < 4 ? 4 : v; g_mFric = v > 99 ? 99 : v
#[test]
fn config_new_clamps_like_c() {
    assert_eq!(MotionConfig::new(0, 50).div, MotionConfig::DIV_MIN);
    assert_eq!(MotionConfig::new(3, 50).div, 4);
    assert_eq!(MotionConfig::new(4, 50).div, 4);
    assert_eq!(MotionConfig::new(500, 50).div, 500);
    assert_eq!(MotionConfig::new(64, 100).friction_pct, MotionConfig::FRICTION_MAX);
    assert_eq!(MotionConfig::new(64, 255).friction_pct, 99);
    assert_eq!(MotionConfig::new(64, 0).friction_pct, 0);
}

#[test]
fn wheel_max_is_15() {
    assert_eq!(WHEEL_MAX, 15);
}

// ---- mouse: hand-worked ----

#[test]
fn not_touching_never_moves() {
    let out = mouse_run(&CFG, &[rpad(false, 0, 0), rpad(false, 5000, 5000)]);
    assert_eq!(out, vec![(0, 0), (0, 0)]);
}

// the first touch frame only records the position (C: rtouch && prt)
#[test]
fn first_touch_frame_does_not_move() {
    let out = mouse_run(&CFG, &[rpad(true, 12_000, -9_000)]);
    assert_eq!(out, vec![(0, 0)]);
}

// delta 640 = exactly one pixel at div 64 (64 * 10)
#[test]
fn delta_of_640_is_one_pixel() {
    let out = mouse_run(&CFG, &[rpad(true, 0, 0), rpad(true, 640, 0)]);
    assert_eq!(out[1], (1, 0));
}

// pad Y up = cursor up = negative HID Y
#[test]
fn y_is_inverted() {
    let out = mouse_run(&CFG, &[rpad(true, 0, 0), rpad(true, 0, 640)]);
    assert_eq!(out[1], (0, -1));
}

// friction 0: no glide, so each frame is delta / 640 plus the carried remainder
#[test]
fn sub_pixel_remainder_carries() {
    let out = mouse_run(
        &NO_GLIDE,
        &[rpad(true, 0, 0), rpad(true, 320, 0), rpad(true, 640, 0), rpad(true, 960, 0)],
    );
    assert_eq!(out, vec![(0, 0), (0, 0), (1, 0), (0, 0)]);
}

#[test]
fn no_glide_stops_on_release() {
    let out = mouse_run(
        &NO_GLIDE,
        &[rpad(true, 0, 0), rpad(true, 6400, 0), rpad(false, 6400, 0), rpad(false, 6400, 0)],
    );
    assert_eq!(out, vec![(0, 0), (10, 0), (0, 0), (0, 0)]);
}

// with friction the cursor keeps gliding after the finger lifts, then stops for good
#[test]
fn glide_continues_then_stops_exactly() {
    let mut frames = vec![rpad(true, 0, 0), rpad(true, 6400, 0)];
    frames.extend(std::iter::repeat_n(rpad(false, 6400, 0), 300));
    let out = mouse_run(&CFG, &frames);
    let after_release: i32 = out[2..].iter().map(|&(x, _)| x as i32).sum();
    assert!(after_release > 0, "should glide");
    assert!(out[250..].iter().all(|&o| o == (0, 0)), "glide must end");
}

// lifting and touching somewhere else must not teleport the cursor
#[test]
fn retouch_elsewhere_does_not_jump() {
    let out = mouse_run(
        &NO_GLIDE,
        &[rpad(true, 0, 0), rpad(false, 0, 0), rpad(true, 30_000, -30_000), rpad(true, 30_000, -30_000)],
    );
    assert!(out.iter().all(|&o| o == (0, 0)), "{out:?}");
}

// both slots dragging +640 each frame add up on the one shared cursor
#[test]
fn slots_sum_onto_one_cursor() {
    let a0 = rpad(true, 0, 0);
    let a1 = rpad(true, 320, 0);
    let mut m = PadMouse::NEW;
    m.step(&[Some(&a0), Some(&a0), None, None], &NO_GLIDE);
    assert_eq!(m.step(&[Some(&a1), Some(&a1), None, None], &NO_GLIDE), (1, 0));
}

#[test]
fn clamps_to_127() {
    let lo = rpad(true, i16::MIN, i16::MAX);
    let hi = rpad(true, i16::MAX, i16::MIN);
    let mut m = PadMouse::NEW;
    m.step(&[Some(&lo), Some(&lo), None, None], &NO_GLIDE);
    // 2 * 65535 / 640 = 204.8 -> clamped
    assert_eq!(m.step(&[Some(&hi), Some(&hi), None, None], &NO_GLIDE), (127, 127));
}

// C clamps AFTER taking the remainder, so the excess past 127 is dropped, not carried
#[test]
fn clamped_excess_is_not_carried() {
    let lo = rpad(true, i16::MIN, 0);
    let hi = rpad(true, i16::MAX, 0);
    let mut m = PadMouse::NEW;
    m.step(&[Some(&lo), Some(&lo), None, None], &NO_GLIDE);
    m.step(&[Some(&hi), Some(&hi), None, None], &NO_GLIDE);
    assert_eq!(m.step(&[Some(&hi), Some(&hi), None, None], &NO_GLIDE), (0, 0));
}

// a slot that goes away (None) loses its velocity at once: no glide from a dead controller
#[test]
fn absent_slot_kills_its_glide() {
    let mut m = PadMouse::NEW;
    m.step(&one(&rpad(true, 0, 0)), &CFG);
    m.step(&one(&rpad(true, 20_000, 0)), &CFG);
    let none = [None, None, None, None];
    assert_eq!(m.step(&none, &CFG), (0, 0));
    assert_eq!(m.step(&none, &CFG), (0, 0));
}

// ...and comes back as a first touch
#[test]
fn absent_slot_returns_as_first_touch() {
    let mut m = PadMouse::NEW;
    m.step(&one(&rpad(true, 0, 0)), &NO_GLIDE);
    m.step(&[None, None, None, None], &NO_GLIDE);
    assert_eq!(m.step(&one(&rpad(true, 30_000, 0)), &NO_GLIDE), (0, 0));
}

// only the right pad and its touch bit drive the mouse
#[test]
fn mouse_ignores_left_pad_and_other_buttons() {
    let mut a = rpad(false, 0, 0);
    a.buttons = Buttons::EMPTY.with(Button::LTrackpadTouch).with(Button::RTrackpadClick);
    let mut b = a;
    b.r_trackpad.x = 20_000;
    b.l_trackpad.x = 20_000;
    assert_eq!(mouse_run(&CFG, &[a, b]), vec![(0, 0), (0, 0)]);
}

// rfLizardRelease: after reset the next touch is a first touch and nothing glides
#[test]
fn reset_clears_everything() {
    let mut m = PadMouse::NEW;
    m.step(&one(&rpad(true, 0, 0)), &CFG);
    m.step(&one(&rpad(true, 6720, 0)), &CFG); // leaves velocity and a remainder
    m.reset();
    assert_eq!(m, PadMouse::NEW);
    assert_eq!(m.step(&one(&rpad(true, 6720, 0)), &CFG), (0, 0));
}

#[test]
fn default_is_new() {
    assert_eq!(PadMouse::default(), PadMouse::NEW);
    assert_eq!(PadScroll::default(), PadScroll::NEW);
}

// ---- scroll: hand-worked ----

#[test]
fn scroll_first_touch_does_nothing() {
    assert_eq!(scroll_run(&CFG, &[lpad(true, 9000)]), vec![0]);
}

// div 64 * 24 = 1536 pad units per notch; pad Y up = wheel up (positive)
#[test]
fn scroll_one_notch_each_way() {
    assert_eq!(scroll_run(&CFG, &[lpad(true, 0), lpad(true, 1536)])[1], 1);
    assert_eq!(scroll_run(&CFG, &[lpad(true, 0), lpad(true, -1536)])[1], -1);
}

#[test]
fn scroll_accumulates_small_moves() {
    let out = scroll_run(&CFG, &[lpad(true, 0), lpad(true, 768), lpad(true, 1536)]);
    assert_eq!(out, vec![0, 0, 1]);
}

// truncation is toward zero both ways
#[test]
fn scroll_negative_truncates_toward_zero() {
    let out = scroll_run(&CFG, &[lpad(true, 0), lpad(true, -768), lpad(true, -1536)]);
    assert_eq!(out, vec![0, 0, -1]);
}

#[test]
fn scroll_clamps_to_wheel_max() {
    let out = scroll_run(&CFG, &[lpad(true, -15_000), lpad(true, 15_000)]);
    assert_eq!(out[1], WHEEL_MAX);
    let out = scroll_run(&CFG, &[lpad(true, 15_000), lpad(true, -15_000)]);
    assert_eq!(out[1], -WHEEL_MAX);
}

// C: `if (!anyTouch) sacc = 0` -- a half notch is forgotten once every pad lifts
#[test]
fn scroll_forgets_partial_notch_on_release() {
    let out = scroll_run(
        &CFG,
        &[lpad(true, 0), lpad(true, 768), lpad(false, 768), lpad(true, 0), lpad(true, 768)],
    );
    assert_eq!(out, vec![0, 0, 0, 0, 0]);
}

// ...but not while another controller is still touching
#[test]
fn scroll_keeps_partial_notch_while_any_pad_touches() {
    let mut s = PadScroll::NEW;
    let (a0, a1) = (lpad(true, 0), lpad(true, 768));
    let still = lpad(true, 0);
    let up = lpad(false, 0);
    s.step(&[Some(&a0), Some(&still), None, None], &CFG);
    assert_eq!(s.step(&[Some(&a1), Some(&still), None, None], &CFG), 0); // 0.5
    assert_eq!(s.step(&[Some(&up), Some(&still), None, None], &CFG), 0); // kept
    let b1 = lpad(true, 768);
    assert_eq!(s.step(&[Some(&up), Some(&b1), None, None], &CFG), 1); // 0.5 + 0.5
}

#[test]
fn scroll_retouch_elsewhere_does_not_jump() {
    let out = scroll_run(&CFG, &[lpad(true, 0), lpad(false, 0), lpad(true, 30_000)]);
    assert_eq!(out, vec![0, 0, 0]);
}

#[test]
fn scroll_absent_slot_returns_as_first_touch() {
    let mut s = PadScroll::NEW;
    s.step(&one(&lpad(true, 0)), &CFG);
    s.step(&[None, None, None, None], &CFG);
    assert_eq!(s.step(&one(&lpad(true, 30_000)), &CFG), 0);
}

#[test]
fn scroll_ignores_x_and_right_pad() {
    let mut a = lpad(true, 0);
    a.buttons = a.buttons.with(Button::RTrackpadTouch);
    let mut b = a;
    b.l_trackpad.x = 20_000;
    b.r_trackpad.y = 20_000;
    assert_eq!(scroll_run(&CFG, &[a, b]), vec![0, 0]);
}

#[test]
fn scroll_divisor_follows_config() {
    let slow = MotionConfig::new(128, 94); // 3072 per notch
    assert_eq!(scroll_run(&slow, &[lpad(true, 0), lpad(true, 1536)])[1], 0);
    assert_eq!(scroll_run(&slow, &[lpad(true, 0), lpad(true, 3072)])[1], 1);
}

#[test]
fn scroll_reset_clears_everything() {
    let mut s = PadScroll::NEW;
    s.step(&one(&lpad(true, 0)), &CFG);
    s.step(&one(&lpad(true, 768)), &CFG);
    s.reset();
    assert_eq!(s, PadScroll::NEW);
    assert_eq!(s.step(&one(&lpad(true, 768)), &CFG), 0);
}

// ---- bit-exact against the C port ----

type Frame = [Option<(bool, i16, i16)>; NUM_SLOTS];

fn mouse_inputs(f: &Frame) -> [Option<ControllerInput>; NUM_SLOTS] {
    f.map(|s| s.map(|(t, x, y)| rpad(t, x, y)))
}

fn scroll_inputs(f: &Frame) -> [Option<ControllerInput>; NUM_SLOTS] {
    f.map(|s| s.map(|(t, _, y)| lpad(t, y)))
}

fn refs(v: &[Option<ControllerInput>; NUM_SLOTS]) -> [Option<&ControllerInput>; NUM_SLOTS] {
    [v[0].as_ref(), v[1].as_ref(), v[2].as_ref(), v[3].as_ref()]
}

fn arb_slot() -> impl Strategy<Value = Option<(bool, i16, i16)>> {
    prop_oneof![
        1 => Just(None),
        6 => (any::<bool>(), -4000i16..4000, -4000i16..4000).prop_map(Some),
        1 => (any::<bool>(), any::<i16>(), any::<i16>()).prop_map(Some),
    ]
}

fn arb_frames() -> impl Strategy<Value = Vec<Frame>> {
    prop::collection::vec(
        (arb_slot(), arb_slot(), arb_slot(), arb_slot()).prop_map(|(a, b, c, d)| [a, b, c, d]),
        1..80,
    )
}

fn arb_cfg() -> impl Strategy<Value = MotionConfig> {
    prop_oneof![
        Just(MotionConfig::DEFAULT),
        (any::<u16>(), any::<u8>()).prop_map(|(d, f)| MotionConfig::new(d, f)),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn mouse_matches_c_bit_for_bit(frames in arb_frames(), cfg in arb_cfg()) {
        let mut m = PadMouse::NEW;
        let mut r = RefMouse::default();
        for (i, f) in frames.iter().enumerate() {
            let inputs = mouse_inputs(f);
            let got = m.step(&refs(&inputs), &cfg);
            let want = r.step(f, cfg.div, cfg.friction_pct);
            prop_assert_eq!(got, want, "frame {}", i);
            prop_assert!((-127..=127).contains(&got.0) && (-127..=127).contains(&got.1));
        }
    }

    #[test]
    fn scroll_matches_c_bit_for_bit(frames in arb_frames(), cfg in arb_cfg()) {
        let mut s = PadScroll::NEW;
        let mut r = RefScroll::default();
        for (i, f) in frames.iter().enumerate() {
            let inputs = scroll_inputs(f);
            let got = s.step(&refs(&inputs), &cfg);
            let want = r.step(f, cfg.div);
            prop_assert_eq!(got, want, "frame {}", i);
            prop_assert!((-WHEEL_MAX..=WHEEL_MAX).contains(&got));
        }
    }

    // glide always dies out: after the last touch, 2000 idle frames end at rest
    // (worst case is friction 99 after 80 full-range swipes: ~1550 frames)
    #[test]
    fn glide_always_ends(frames in arb_frames(), cfg in arb_cfg()) {
        let mut m = PadMouse::NEW;
        for f in &frames {
            let inputs = mouse_inputs(f);
            m.step(&refs(&inputs), &cfg);
        }
        let idle = rpad(false, 0, 0);
        let all = [Some(&idle); NUM_SLOTS];
        let mut last = (1, 1);
        for _ in 0..2000 {
            last = m.step(&all, &cfg);
        }
        prop_assert_eq!(last, (0, 0));
    }
}

// =====================================================================
// Reference: a line-for-line port of the C, f32 ops in C's order.
// =====================================================================

#[derive(Default)]
struct RefMouse {
    prx: [i32; NUM_SLOTS],
    pry: [i32; NUM_SLOTS],
    prt: [bool; NUM_SLOTS],
    vx: [f32; NUM_SLOTS],
    vy: [f32; NUM_SLOTS],
    rmx: f32,
    rmy: f32,
}

impl RefMouse {
    #[allow(clippy::needless_range_loop)] // indexed on purpose: mirrors the C loop
    fn step(&mut self, f: &Frame, div: u16, fric: u8) -> (i8, i8) {
        let k = fric as f32 / 100.0; // float f = g_mFric / 100.0f
        let (mut sumx, mut sumy) = (0.0f32, 0.0f32);
        for s in 0..NUM_SLOTS {
            let Some((touch, rx, ry)) = f[s] else {
                self.vx[s] = 0.0;
                self.vy[s] = 0.0;
                self.prt[s] = false;
                continue;
            };
            let (rx, ry) = (rx as i32, ry as i32);
            if touch && self.prt[s] {
                self.vx[s] += (rx - self.prx[s]) as f32;
                self.vy[s] += (ry - self.pry[s]) as f32;
            }
            if touch {
                self.prx[s] = rx;
                self.pry[s] = ry;
            }
            self.prt[s] = touch;
            sumx += self.vx[s];
            sumy += self.vy[s];
            self.vx[s] *= k;
            self.vy[s] *= k;
            if self.vx[s] > -1.0 && self.vx[s] < 1.0 {
                self.vx[s] = 0.0;
            }
            if self.vy[s] > -1.0 && self.vy[s] < 1.0 {
                self.vy[s] = 0.0;
            }
        }
        let d = (div as i32 * 10) as f32;
        let mxf = sumx / d + self.rmx;
        let myf = -(sumy / d) + self.rmy;
        let dx = mxf as i32;
        let dy = myf as i32;
        self.rmx = mxf - dx as f32;
        self.rmy = myf - dy as f32;
        (dx.clamp(-127, 127) as i8, dy.clamp(-127, 127) as i8)
    }
}

#[derive(Default)]
struct RefScroll {
    ply: [i32; NUM_SLOTS],
    plt: [bool; NUM_SLOTS],
    sacc: f32,
}

impl RefScroll {
    #[allow(clippy::needless_range_loop)]
    fn step(&mut self, f: &Frame, div: u16) -> i8 {
        let mut any = false;
        for s in 0..NUM_SLOTS {
            let Some((touch, _, ly)) = f[s] else {
                self.plt[s] = false;
                continue;
            };
            let ly = ly as i32;
            if touch && self.plt[s] {
                self.sacc += (ly - self.ply[s]) as f32 / (div as i32 * 24) as f32;
            }
            if touch {
                self.ply[s] = ly;
                any = true;
            }
            self.plt[s] = touch;
        }
        if !any {
            self.sacc = 0.0;
        }
        let dw = self.sacc as i32;
        self.sacc -= dw as f32;
        dw.clamp(-15, 15) as i8
    }
}
