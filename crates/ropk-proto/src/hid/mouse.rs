use core::ops::BitOr;

pub const REPORT_ID_MOUSE: u8 = 0x40;
pub const MOUSE_BODY_LEN: usize = 5;
pub const MOUSE_REPORT_LEN: usize = MOUSE_BODY_LEN + 1;
pub const AXIS_MIN: i8 = -127;
pub const AXIS_MAX: i8 = 127;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct MouseButtons(u8);

impl MouseButtons {
    pub const EMPTY: Self = Self(0);
    pub const LEFT: Self = Self(1 << 0);   // btn 1 0x1
    pub const RIGHT: Self = Self(1 << 1);  // btn 2 0x2
    pub const MIDDLE: Self = Self(1 << 2); // btn 3 0x4
    pub const ALL: Self = Self(Self::LEFT.0 | Self::RIGHT.0 | Self::MIDDLE.0); // 0x7

    pub const fn from_bits(bits: u8) -> Self {
        Self(bits & Self::ALL.0)
    }

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn union(self, other: Self) -> Self {
        self.with(other)
    }

    pub const fn remove(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl BitOr for MouseButtons {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        self.union(rhs)
    }
}

pub fn clamp_axis(v: i32) -> i8 {
    if v > AXIS_MAX as i32 {
        AXIS_MAX
    } else if v < AXIS_MIN as i32 {
        AXIS_MIN
    } else {
        v as i8
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct MouseReport {
    pub buttons: MouseButtons,
    pub x: i8,
    pub y: i8,
    pub wheel: i8,
    pub pan: i8,
}

impl MouseReport {
    pub const NEUTRAL: Self = Self {
        buttons: MouseButtons::EMPTY,
        x: 0,
        y: 0,
        wheel: 0,
        pan: 0,
    };

    pub fn is_neutral(&self) -> bool {
        *self == Self::NEUTRAL
    }

    pub fn has_motion(&self) -> bool {
        self.x != 0 || self.y != 0 || self.wheel != 0 || self.pan != 0
    }

    pub fn encode(&self) -> [u8; MOUSE_REPORT_LEN] {
        [
            REPORT_ID_MOUSE,
            self.buttons.bits() & MouseButtons::ALL.bits(),
            self.x.max(AXIS_MIN) as u8,
            self.y.max(AXIS_MIN) as u8,
            self.wheel.max(AXIS_MIN) as u8,
            self.pan.max(AXIS_MIN) as u8,
        ]
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < MOUSE_REPORT_LEN || bytes[0] != REPORT_ID_MOUSE {
            return None;
        }
        Some(Self {
            buttons: MouseButtons::from_bits(bytes[1]),
            x: bytes[2] as i8,
            y: bytes[3] as i8,
            wheel: bytes[4] as i8,
            pan: bytes[5] as i8,
        })
    }
}