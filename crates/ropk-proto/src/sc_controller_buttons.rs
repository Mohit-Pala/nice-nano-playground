// button defs - wanted to keep this separated
use core::ops::{BitAnd, BitOr};

// mask for all real sc buttons, c code has support for virtual ps4 and ps5 buttons, do this later
// todo - 0 .. 29 bits
pub const PHYSICAL_SC_BUTTONS_MASK:u32 = (1 << 30) - 1;

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]

pub enum Button {
    // shit in a regular ass controller

    LStick = 1 << 15,
    RStick = 1 << 5,

    A = 1 << 0,
    B = 1 << 1,
    X = 1 << 2,
    Y = 1 << 3,

    DpadDown = 1 << 10,
    DpadRight = 1 << 11,
    DpadLeft = 1 << 12,
    DpadUp = 1 << 13,


    Lb = 1 << 19,
    LT = 1 << 27,
    Rb = 1 << 9,
    RT = 1 << 23,

    // select and start / menu, refer to the image 
    Select = 1 << 6,
    Start = 1 << 14,

    // shit not in a regular ass controller

    // back buttons - refer to the image on what these mean
    L4 = 1 << 17,
    L5 = 1 << 18,
    R4 = 1 << 7,
    R5 = 1 << 8,

    // steam shit
    SteamQuickAccess = 1 << 4,
    Steam = 1 << 16,

    // trackpads and capacitive sticks and grip sensors
    LStickTouch = 1 << 24,
    RStickTouch = 1 << 20,

    LTrackpadTouch = 1 << 25,
    LTrackpadClick = 1 << 26,
    RTrackpadTouch = 1 << 21,
    RTrackpadClick = 1 << 22,

    LGripTouch = 1 << 29,
    RGripTouch = 1 << 28,
}

impl Button {
    // dont touch this array, stolen from triton.h
    pub const ALL: [Button; 30] = [
        Button::A,
        Button::B,
        Button::X,
        Button::Y,
        Button::SteamQuickAccess,
        Button::RStick,
        Button::Select,
        Button::R4,
        Button::R5,
        Button::Rb,
        Button::DpadDown,
        Button::DpadRight,
        Button::DpadLeft,
        Button::DpadUp,
        Button::Start,
        Button::LStick,
        Button::Steam,
        Button::L4,
        Button::L5,
        Button::Lb,
        Button::RStickTouch,
        Button::RTrackpadTouch,
        Button::RTrackpadClick,
        Button::RT,
        Button::LStickTouch,
        Button::LTrackpadTouch,
        Button::LTrackpadClick,
        Button::LT,
        Button::RGripTouch,
        Button::LGripTouch,
    ];

    pub const fn bit(self) -> u32 {
        self as u32
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Buttons(u32);

impl Buttons {
    pub const EMPTY: Buttons = Buttons(0);

    pub const PHYSICAL: Buttons = Buttons(PHYSICAL_SC_BUTTONS_MASK);

    pub const BACK4: Buttons = Buttons::EMPTY
        .with(Button::L4)
        .with(Button::L5)
        .with(Button::R4)
        .with(Button::R5);

    pub const GRIPS: Buttons = Buttons::EMPTY
        .with(Button::LGripTouch)
        .with(Button::RGripTouch);

    pub const fn from_bits(bits: u32) -> Self {
        Buttons(bits)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn has(self, b: Button) -> bool {
        self.0 & b.bit() != 0
    }

    pub const fn contains(self, other: Buttons) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn intersects(self, other: Buttons) -> bool {
        self.0 & other.0 != 0
    }

    pub const fn with(self, b: Button) -> Self {
        Buttons(self.0 | b.bit())
    }

    pub const fn union(self, other: Buttons) -> Self {
        Buttons(self.0 | other.0)
    }

    pub const fn remove(self, other: Buttons) -> Self {
        Buttons(self.0 & !other.0)
    }

    pub fn iter(self) -> impl Iterator<Item = Button> {
        Button::ALL.into_iter().filter(move |b| self.has(*b))
    }
}

impl From<Button> for Buttons {
    fn from(b: Button) -> Self {
        Buttons(b.bit())
    }
}

impl<R: Into<Buttons>> BitOr<R> for Button {
    type Output = Buttons;
    fn bitor(self, rhs: R) -> Buttons {
        Buttons::from(self).union(rhs.into())
    }
}

impl<R: Into<Buttons>> BitOr<R> for Buttons {
    type Output = Buttons;
    fn bitor(self, rhs: R) -> Buttons {
        self.union(rhs.into())
    }
}

impl<R: Into<Buttons>> BitAnd<R> for Buttons {
    type Output = Buttons;
    fn bitand(self, rhs: R) -> Buttons {
        Buttons(self.0 & rhs.into().0)
    }
}
