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
    lx: i16,
    ly: i16,
    rx:i16,
    ry: i16,
}

pub struct Trackpad {
    pub x,
    pub y, 
    pub press.
}

