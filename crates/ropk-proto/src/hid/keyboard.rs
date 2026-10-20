use core::ops::{BitOr, BitOrAssign};

pub const REPORT_ID_KEYBOARD: u8 = 0x41;
pub const KEYBOARD_BODY_LEN: usize = 8;
pub const KEYBOARD_REPORT_LEN: usize = KEYBOARD_BODY_LEN + 1;
pub const MAX_KEYS: usize = 6;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Modifiers(u8);

impl Modifiers {
    pub const EMPTY: Self = Self(0);
    pub const LCTRL: Self = Self(1 << 0);
    pub const LSHIFT: Self = Self(1 << 1);
    pub const LALT: Self = Self(1 << 2);
    pub const LGUI: Self = Self(1 << 3);
    pub const RCTRL: Self = Self(1 << 4);
    pub const RSHIFT: Self = Self(1 << 5);
    pub const RALT: Self = Self(1 << 6);
    pub const RGUI: Self = Self(1 << 7);

    pub const fn from_bits(bits: u8) -> Self {
        Self(bits)
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

impl BitOr for Modifiers {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        self.union(rhs)
    }
}

impl BitOrAssign for Modifiers {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = self.union(rhs);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Key(u8);

impl Key {
    pub const USAGE_MIN: u8 = 0x04;
    pub const USAGE_MAX: u8 = 0x65;

    pub const NONE: Self = Self(0x00);
    pub const A: Self = Self(0x04);
    pub const O: Self = Self(0x12);
    pub const ENTER: Self = Self(0x28);
    pub const ESCAPE: Self = Self(0x29);
    pub const TAB: Self = Self(0x2B);
    pub const PAGE_UP: Self = Self(0x4B);
    pub const DELETE: Self = Self(0x4C);
    pub const PAGE_DOWN: Self = Self(0x4E);
    pub const ARROW_RIGHT: Self = Self(0x4F);
    pub const ARROW_LEFT: Self = Self(0x50);
    pub const ARROW_DOWN: Self = Self(0x51);
    pub const ARROW_UP: Self = Self(0x52);

    pub const fn from_usage(u: u8) -> Option<Self> {
        if u >= Self::USAGE_MIN && u <= Self::USAGE_MAX {
            Some(Self(u))
        } else {
            None
        }
    }

    pub const fn usage(self) -> u8 {
        self.0
    }

    pub const fn is_none(self) -> bool {
        self.0 == 0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct KeyboardReport {
    pub modifiers: Modifiers,
    keys: [Key; MAX_KEYS],
    len: u8,
}

impl KeyboardReport {
    pub const NEUTRAL: Self = Self {
        modifiers: Modifiers::EMPTY,
        keys: [Key::NONE; MAX_KEYS],
        len: 0,
    };

    pub fn press(&mut self, key: Key) -> bool {
        if key.is_none() {
            return true;
        }
        let len = self.len as usize;
        for &k in &self.keys[..len] {
            if k == key {
                return true;
            }
        }
        if len >= MAX_KEYS {
            return false;
        }
        self.keys[len] = key;
        self.len += 1;
        true
    }

    pub fn contains(&self, key: Key) -> bool {
        if key.is_none() {
            return false;
        }
        let len = self.len as usize;
        self.keys[..len].contains(&key)
    }

    pub fn keys(&self) -> &[Key] {
        &self.keys[..self.len as usize]
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn is_full(&self) -> bool {
        self.len as usize == MAX_KEYS
    }

    pub fn is_neutral(&self) -> bool {
        *self == Self::NEUTRAL
    }

    pub fn encode(&self) -> [u8; KEYBOARD_REPORT_LEN] {
        let mut out = [0u8; KEYBOARD_REPORT_LEN];
        out[0] = REPORT_ID_KEYBOARD;
        out[1] = self.modifiers.bits();
        out[2] = 0x00;
        let len = self.len as usize;
        for i in 0..len {
            out[3 + i] = self.keys[i].usage();
        }
        out
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < KEYBOARD_REPORT_LEN || bytes[0] != REPORT_ID_KEYBOARD {
            return None;
        }
        let mut report = Self::NEUTRAL;
        report.modifiers = Modifiers::from_bits(bytes[1]);
        for &b in &bytes[3..3 + MAX_KEYS] {
            if b != 0 {
                let key = Key::from_usage(b)?;
                report.press(key);
            }
        }
        Some(report)
    }
}
