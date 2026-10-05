use crate::sc_controller_buttons::Buttons;

// rf_link.cpp - 161 - slot alive if < 300ms
pub const LINK_TIMEOUT_MS: u64 = 300;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Triggers {
    pub l: u16,
    pub r: u16,
}

// this is disgusting, fuck the rust formatter
pub fn trigger_u8(raw: u16) -> u8 {
    let scaled = raw >> 7;
    if scaled > 255 { 255 } else { scaled as u8 }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Sticks {
    pub lx: i16,
    pub ly: i16,
    pub rx: i16,
    pub ry: i16,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Trackpad {
    pub x: i16,
    pub y: i16,
    pub press: i16,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Imu {
    pub timestamp: u32,
    pub accel: [i16; 3],
    pub gyro: [i16; 3],
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ControllerInput {
    pub buttons: Buttons,
    pub triggers: Triggers,
    pub sticks: Sticks,
    pub l_trackpad: Trackpad,
    pub r_trackpad: Trackpad,
    pub imu: Option<Imu>,
}

impl ControllerInput {
    pub const NEUTRAL: Self = Self {
        buttons: Buttons::EMPTY,
        triggers: Triggers { l: 0, r: 0 },
        sticks: Sticks {
            lx: 0,
            ly: 0,
            rx: 0,
            ry: 0,
        },
        l_trackpad: Trackpad {
            x: 0,
            y: 0,
            press: 0,
        },
        r_trackpad: Trackpad {
            x: 0,
            y: 0,
            press: 0,
        },
        imu: None,
    };

    pub fn is_neutral(&self) -> bool {
        self.buttons.is_empty()
            && self.triggers == Triggers::default()
            && self.sticks == Sticks::default()
            && self.l_trackpad == Trackpad::default()
            && self.r_trackpad == Trackpad::default()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Sample {
    pub input: ControllerInput,
    pub received_ms: u64,
}

impl Sample {
    pub fn age_ms(&self, now_ms: u64) -> u64 {
        now_ms.saturating_sub(self.received_ms)
    }

    pub fn is_fresh(&self, now_ms: u64) -> bool {
        self.age_ms(now_ms) < LINK_TIMEOUT_MS
    }

    pub fn current(&self, now_ms: u64) -> ControllerInput {
        if self.is_fresh(now_ms) {
            self.input
        } else {
            ControllerInput::NEUTRAL
        }
    }
}
