use core::ops::BitOr;

use crate::input::ControllerInput;
use crate::sc_controller_buttons::Button;

pub const STICK_THRESHOLD: i16 = 12_000;
pub const PHYSICAL_MASK: u32 = 0x0FFF_FFFF;
pub const VIRTUAL_MASK: u32 = 0xF000_0000;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum StickDir {  // name in c code
    Right = 1 << 28, // LZ_BTN_LSTICK_RT
    Left = 1 << 29,  // LZ_BTN_LSTICK_LF
    Down = 1 << 30,  // LZ_BTN_LSTICK_DN
    Up = 1 << 31,    // LZ_BTN_LSTICK_UP
}

impl StickDir {
    pub const ALL: [StickDir; 4] = [
        StickDir::Right,
        StickDir::Left,
        StickDir::Down,
        StickDir::Up,
    ];

    pub const fn bit(self) -> u32 {
        self as u32
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct LizardButtons(u32);

impl LizardButtons {
    pub const EMPTY: Self = Self(0);

    pub const fn from_input(input: &ControllerInput) -> Self {
        let mut bits = input.buttons.bits() & PHYSICAL_MASK;
        if input.sticks.lx > STICK_THRESHOLD {
            bits |= StickDir::Right.bit();
        } else if input.sticks.lx < -STICK_THRESHOLD {
            bits |= StickDir::Left.bit();
        }
        if input.sticks.ly > STICK_THRESHOLD {
            bits |= StickDir::Up.bit();
        } else if input.sticks.ly < -STICK_THRESHOLD {
            bits |= StickDir::Down.bit();
        }
        Self(bits)
    }

    pub const fn button(b: Button) -> Self {
        assert!(b.bit() & VIRTUAL_MASK == 0);
        Self(b.bit())
    }

    pub const fn stick(d: StickDir) -> Self {
        Self(d.bit())
    }

    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub const fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl BitOr for LizardButtons {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}
