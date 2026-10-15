// regarding radio addresses

use crate::bond::UUID_LEN;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RadioAddr {
    pub base: [u8; 4],
    pub prefix: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AddrRegs {
    pub base0: u32,
    pub prefix0: u32,
}

// values from radio h
// ibex is the base and 0x10 is the prefix
pub const DISCOVERY_ADDR: RadioAddr = RadioAddr {
    base: *b"ibex",
    prefix: 0x10,
};

pub const DISCOVERY_CHANNEL: u8 = 0x2;
pub const DEFAULT_SESSION_CHANNEL: u8 = 18;

// values declared here are some random ass values in the c code
// todo: look into these values later on
// radio cpp line 62, 67 and 75
pub const BASE_BYTE_XOR: u8 = 0x5A; // base byte of 00 or ff is xored with this
pub const PREFIX_REPLACE: u8 = 0x5C; // prefix of 00 or ff is replaced with this
pub const DISCOVERY_COLLISION: u8 = 0x80; // xored into base 0 if result is ibex 


// bitrev8 - was a helper in the radio create - moved here
pub const fn bitrev8(x: u8) -> u8 {
    x.reverse_bits()
}

impl RadioAddr {
    pub const fn to_regs(self) -> AddrRegs {
        AddrRegs {
            base0: u32::from_be_bytes([
                bitrev8(self.base[0]),
                bitrev8(self.base[1]),
                bitrev8(self.base[2]),
                bitrev8(self.base[3]),
            ]),
            prefix0: bitrev8(self.prefix) as u32,
        }
    }

    pub fn session_addr(uuid: &[u8; UUID_LEN], device_id: [u32; 2]) -> RadioAddr {
        
    }
}