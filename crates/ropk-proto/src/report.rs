// from rflink cpp 

pub const REPORT_ID_INPUT_LEGACY: u8 = 0x45; // original 46 byte 
pub const REPORT_ID_INPUT: u8 = 0x42; // Post july firmware - was called v2 in c code
pub const REPORT_ID_STATUS: u8 = 0x43; // regaRDING BATTERTY 
pub const REPORT_ID_STATUS_EVENT : u8= 0x44; // STATYUS BNODY EVERY 2 SECS

pub const INPUT_MIN_LEN: u8 = 0x1E;
pub const INPUT_FULL_LEN: u8 = 0x2E;
pub const STATUS_MIN_LEN: u8 = 0x3;

// Field offsets, s16 off adds 2 
pub const OFF_SEQ: usize = 0x01;
pub const OFF_BUTTONS: usize = 0x02;

pub const OFF_LT: usize = 0x06;
pub const OFF_RT: usize = 0x08;

pub const OFF_LX: usize = 0x0A;
pub const OFF_LY: usize = 0x0C;
pub const OFF_RX: usize = 0x0E;
pub const OFF_RY: usize = 0x10;

pub const OFF_LPAD_X: usize = 0x12;
pub const OFF_LPAD_Y: usize = 0x14;
pub const OFF_LPAD_PRESS: usize = 0x16;

pub const OFF_RPAD_X: usize = 0x18;
pub const OFF_RPAD_Y: usize = 0x1A;
pub const OFF_RPAD_PRESS: usize = 0x1C;

pub const OFF_IMU_TIMESTAMP: usize = 0x1E;
pub const OFF_ACCEL_X: usize = 0x22;
pub const OFF_ACCEL_Y: usize = 0x24;
pub const OFF_ACCEL_Z: usize = 0x26;
pub const OFF_GYRO_X: usize = 0x28;
pub const OFF_GYRO_Y: usize = 0x2A;
pub const OFF_GYRO_Z: usize = 0x2C;

