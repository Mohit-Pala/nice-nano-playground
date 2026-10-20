use core::ops::{BitOr, BitOrAssign};

pub const REPORT_ID_CONSUMER: u8 = 0x03;
pub const CONSUMER_BODY_LEN: usize = 1;
pub const CONSUMER_REPORT_LEN: usize = CONSUMER_BODY_LEN + 1;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ConsumerControls(u8);

impl ConsumerControls {
    pub const EMPTY: Self = Self(0);
    pub const VOL_UP: Self = Self(1 << 0);
    pub const VOL_DOWN: Self = Self(1 << 1);
    pub const ALL: Self = Self((1 << 0) | (1 << 1));

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

impl BitOr for ConsumerControls {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        self.union(rhs)
    }
}

impl BitOrAssign for ConsumerControls {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = self.union(rhs);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ConsumerReport {
    pub controls: ConsumerControls,
}

impl ConsumerReport {
    pub const NEUTRAL: Self = Self {
        controls: ConsumerControls::EMPTY,
    };

    pub fn is_neutral(&self) -> bool {
        *self == Self::NEUTRAL
    }

    pub fn encode(&self) -> [u8; CONSUMER_REPORT_LEN] {
        [REPORT_ID_CONSUMER, self.controls.bits() & ConsumerControls::ALL.bits()]
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < CONSUMER_REPORT_LEN || bytes[0] != REPORT_ID_CONSUMER {
            return None;
        }
        Some(Self {
            controls: ConsumerControls::from_bits(bytes[1]),
        })
    }
}
