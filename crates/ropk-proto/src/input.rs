#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Triggers {
    pub l: u16,
    pub r: u16,
}

pub fn trigger_u8(raw: u16) -> u8 {
    let scaled = raw >> 7;
    if scaled > 255 {
        255
    } else {
        scaled as u8
    }
}

pub struct Sticks {
    pub lx: i16,
    pub ly: i16,
    pub rx:i16,
    pub ry: i16,
}

pub struct Trackpad {
    pub x: i16,
    pub y: i16, 
    pub press: i16,
}

