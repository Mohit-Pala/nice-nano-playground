// from rflink cpp 

pub const REPORT_ID_INPUT_LEGACY: u8 = 0x45; // original 46 byte 
pub const REPORT_ID_INPUT: u8 = 0x42; // Post july firmware - was called v2 in c code
pub const REPORT_ID_STATUS: u8 = 0x43; // regaRDING BATTERTY 
pub const REPORT_ID_STATUS_EVENT : u8= 0x44; // STATYUS BNODY EVERY 2 SECS

pub const INPUT_MIN_LEN: u8 = 0x1E;
pub const INPUT_FULL_LEN: u8 = 0x2E;
pub const STATUS_MIN_LEN: u8 = 0x3;

// Field offsets - from that bigass if statement in rflink

