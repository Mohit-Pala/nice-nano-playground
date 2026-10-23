use crate::bond::NUM_SLOTS;
use crate::hid::mouse::clamp_axis;
use crate::input::ControllerInput;
use crate::sc_controller_buttons::Button;

pub const WHEEL_MAX: i8 = 15;
pub const MOUSE_SCALE: i32 = 10;
pub const SCROLL_SCALE: i32 = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct MotionConfig {
    pub div: u16,
    pub friction_pct: u8,
}

impl MotionConfig {
    pub const DEFAULT: Self = Self {
        div: 64,
        friction_pct: 94,
    };
    pub const DIV_MIN: u16 = 4;
    pub const FRICTION_MAX: u8 = 99;

    pub const fn new(div: u16, friction_pct: u8) -> Self {
        let div = if div < Self::DIV_MIN {
            Self::DIV_MIN
        } else {
            div
        };
        let friction_pct = if friction_pct > Self::FRICTION_MAX {
            Self::FRICTION_MAX
        } else {
            friction_pct
        };
        Self { div, friction_pct }
    }
}

impl Default for MotionConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PadMouse {
    prev_x: [i32; NUM_SLOTS],
    prev_y: [i32; NUM_SLOTS],
    touching: [bool; NUM_SLOTS],
    vx: [f32; NUM_SLOTS],
    vy: [f32; NUM_SLOTS],
    rem_x: f32,
    rem_y: f32,
}

impl PadMouse {
    pub const NEW: Self = Self {
        prev_x: [0; NUM_SLOTS],
        prev_y: [0; NUM_SLOTS],
        touching: [false; NUM_SLOTS],
        vx: [0.0; NUM_SLOTS],
        vy: [0.0; NUM_SLOTS],
        rem_x: 0.0,
        rem_y: 0.0,
    };

    pub fn reset(&mut self) {
        *self = Self::NEW;
    }

    pub fn step(
        &mut self,
        slots: &[Option<&ControllerInput>; NUM_SLOTS],
        cfg: &MotionConfig,
    ) -> (i8, i8) {
        let k = cfg.friction_pct as f32 / 100.0;
        let mut sum_x = 0.0f32;
        let mut sum_y = 0.0f32;

        for s in 0..NUM_SLOTS {
            let Some(input) = slots[s] else {
                self.vx[s] = 0.0;
                self.vy[s] = 0.0;
                self.touching[s] = false;
                continue;
            };

            let touch = input.buttons.has(Button::RTrackpadTouch);
            let rx = input.r_trackpad.x as i32;
            let ry = input.r_trackpad.y as i32;

            if touch && self.touching[s] {
                self.vx[s] += (rx - self.prev_x[s]) as f32;
                self.vy[s] += (ry - self.prev_y[s]) as f32;
            }
            if touch {
                self.prev_x[s] = rx;
                self.prev_y[s] = ry;
            }
            self.touching[s] = touch;

            sum_x += self.vx[s];
            sum_y += self.vy[s];

            self.vx[s] *= k;
            self.vy[s] *= k;

            if self.vx[s] > -1.0 && self.vx[s] < 1.0 {
                self.vx[s] = 0.0;
            }
            if self.vy[s] > -1.0 && self.vy[s] < 1.0 {
                self.vy[s] = 0.0;
            }
        }

        let d = (cfg.div as i32 * MOUSE_SCALE) as f32;
        let mx = sum_x / d + self.rem_x;
        let my = -(sum_y / d) + self.rem_y;
        let dx = mx as i32;
        let dy = my as i32;
        self.rem_x = mx - dx as f32;
        self.rem_y = my - dy as f32;

        (clamp_axis(dx), clamp_axis(dy))
    }
}

impl Default for PadMouse {
    fn default() -> Self {
        Self::NEW
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PadScroll {
    prev_y: [i32; NUM_SLOTS],
    touching: [bool; NUM_SLOTS],
    acc: f32,
}

impl PadScroll {
    pub const NEW: Self = Self {
        prev_y: [0; NUM_SLOTS],
        touching: [false; NUM_SLOTS],
        acc: 0.0,
    };

    pub fn reset(&mut self) {
        *self = Self::NEW;
    }

    pub fn step(
        &mut self,
        slots: &[Option<&ControllerInput>; NUM_SLOTS],
        cfg: &MotionConfig,
    ) -> i8 {
        let mut any = false;

        for s in 0..NUM_SLOTS {
            let Some(input) = slots[s] else {
                self.touching[s] = false;
                continue;
            };

            let touch = input.buttons.has(Button::LTrackpadTouch);
            let ly = input.l_trackpad.y as i32;

            if touch && self.touching[s] {
                self.acc += (ly - self.prev_y[s]) as f32 / (cfg.div as i32 * SCROLL_SCALE) as f32;
            }
            if touch {
                self.prev_y[s] = ly;
                any = true;
            }
            self.touching[s] = touch;
        }

        if !any {
            self.acc = 0.0;
        }

        let dw = self.acc as i32;
        self.acc -= dw as f32;
        dw.clamp(-(WHEEL_MAX as i32), WHEEL_MAX as i32) as i8
    }
}

impl Default for PadScroll {
    fn default() -> Self {
        Self::NEW
    }
}
