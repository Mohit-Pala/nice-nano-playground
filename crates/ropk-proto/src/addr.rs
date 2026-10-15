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

// hash constants - golden ratio, MurmurHash3, and xxHash32
pub const HASH_GOLDEN: u32 = 0x9E37_79B1;
pub const HASH_MURMUR_1: u32 = 0x85EB_CA6B;
pub const HASH_MURMUR_2: u32 = 0xC2B2_AE35;
pub const HASH_XXHASH_1: u32 = 0x27D4_EB2F;
pub const HASH_XXHASH_2: u32 = 0x1656_67B1;

// radio constrs 
pub const INVALID_BYTE_00: u8 = 0x00;
pub const INVALID_BYTE_FF: u8 = 0xFF;
pub const PREFIX_SHIFT: u32 = 16;

#[inline]
pub const fn is_invalid_addr_byte(b: u8) -> bool {
    b == INVALID_BYTE_00 || b == INVALID_BYTE_FF
}

// bitrev8 - was a helper in the radio create - moved here
pub const fn bitrev8(x: u8) -> u8 {
    x.reverse_bits()
}

fn hash_session(uuid: &[u8; UUID_LEN], [dev0, dev1]: [u32; 2]) -> (u32, u32) {
    if *uuid != [0; UUID_LEN] {
        let [u0, u1, u2, u3, u4, u5, u6, u7] = uuid.map(|b| b as u32);
        (
            u0.wrapping_mul(HASH_GOLDEN)
                ^ u2.wrapping_mul(HASH_MURMUR_1)
                ^ u4.wrapping_mul(HASH_MURMUR_2)
                ^ u6
                ^ dev0,
            u1.wrapping_mul(HASH_XXHASH_1)
                ^ u3.wrapping_mul(HASH_XXHASH_2)
                ^ u5
                ^ u7.wrapping_mul(HASH_GOLDEN)
                ^ dev1,
        )
    } else {
        (
            dev0.wrapping_mul(HASH_GOLDEN) ^ dev1,
            dev1.wrapping_mul(HASH_MURMUR_1) ^ dev0,
        )
    }
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
        let (h, h2) = hash_session(uuid, device_id);

        let mut base = h.to_le_bytes();
        for b in &mut base {
            if is_invalid_addr_byte(*b) {
                *b ^= BASE_BYTE_XOR;
            }
        }

        let mut prefix = (h2 >> PREFIX_SHIFT) as u8;
        if is_invalid_addr_byte(prefix) {
            prefix = PREFIX_REPLACE;
        }

        if base == DISCOVERY_ADDR.base {
            base[0] ^= DISCOVERY_COLLISION;
        }

        RadioAddr { base, prefix }
    }
}