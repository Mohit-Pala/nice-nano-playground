// from bonds.h puck_hid.cpp
pub const NUM_SLOTS: usize = 4;
pub const PUCK_UUID_LEN: usize = 4;
pub const IBEX_UUID_LEN: usize = 4;
pub const SERIAL_LEN: usize = 16; // reeses puffs are goated
pub const UUID_LEN: usize = PUCK_UUID_LEN + IBEX_UUID_LEN; // 4 + 4 - rf link cpp - 
pub const BOND_LEN: usize = 24;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
// fuckn hate the newtype pattern
pub struct Bond([u8; BOND_LEN]);
pub enum BondWrite {
    TooShort,
    Clear,
    Set(Bond)
}

pub fn bond_from_bytes(bytes: &[u8]) -> BondWrite {
    let Some(head) = bytes.first_chunk::<BOND_LEN>() else {
        return BondWrite::TooShort;
    };

    if head == [0; BOND_LEN] {
        return BondWrite::Clear;
    }
    BondWrite::Set(Bond(head))
}

impl Bond {
    pub fn as_bytes(&self) -> &[u8; BOND_LEN] {
        &self.0
    } 
    pub fn puck_uuid(&self) -> &[u8; PUCK_UUID_LEN] {
        self.0.first_chunk().unwrap()
    }
    pub fn ibex_uuid(&self) -> &[u8; IBEX_UUID_LEN] {
        self.0[PUCK_UUID_LEN..UUID_LEN].try_into().unwrap()
    }
    pub fn uuid(&self) -> &[u8; UUID_LEN] {
        self.0.first_chunk().unwrap()
    }
    pub fn serial(&self) -> &[u8; SERIAL_LEN] {
        self.0.last_chunk().unwrap()
    }
}
