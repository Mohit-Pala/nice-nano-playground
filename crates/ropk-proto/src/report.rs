use crate::bytes::{i16_le, u16_le, u32_le};
use crate::input::{ControllerInput, Imu, Trackpad, Sticks, Triggers};
use crate::sc_controller_buttons::Buttons;

// from rflink cpp 

pub const REPORT_ID_INPUT_LEGACY: u8 = 0x45; // original 46 byte 
pub const REPORT_ID_INPUT: u8 = 0x42; // Post july firmware - was called v2 in c code
pub const REPORT_ID_STATUS: u8 = 0x43; // regaRDING BATTERTY 
pub const REPORT_ID_STATUS_EVENT : u8= 0x44; // STATYUS BNODY EVERY 2 SECS

pub const INPUT_MIN_LEN: usize = 0x1E;
pub const INPUT_FULL_LEN: usize = 0x2E;
pub const STATUS_MIN_LEN: usize = 0x3;

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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct InputReport<'a> {
    pub id: u8, // 0x45 or 0x42, use the right id
    pub seq: u8,
    pub input: ControllerInput,
    pub raw: &'a [u8], // entire TLV value
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StatusReport<'a> {
    pub charge_state: u8, 
    pub battery_pct: u8,
    pub raw: &'a [u8],
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Report<'a> {
    Input(InputReport<'a>),
    Status(StatusReport<'a>),
    StatusEvent(&'a [u8]),
    Unknown { id: u8, raw: &'a [u8] },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReportError {
    Empty,
    InputTooShort { id: u8, len: usize },
    StatusTooShort { len: usize },
}

fn decode_imu(r: &[u8]) -> Option<Imu> {
    Some(Imu {
        timestamp: u32_le(r, OFF_IMU_TIMESTAMP)?,
        accel: [
            i16_le(r, OFF_ACCEL_X)?,
            i16_le(r, OFF_ACCEL_Y)?,
            i16_le(r, OFF_ACCEL_Z)?,
        ],
        gyro: [
            i16_le(r, OFF_GYRO_X)?,
            i16_le(r, OFF_GYRO_Y)?,
            i16_le(r, OFF_GYRO_Z)?,
        ],
    })
}

fn decode_buttons_triggers_trackpads(r: &[u8]) -> Option<ControllerInput> {
    let buttons = Buttons::from_bits(u32_le(r, OFF_BUTTONS)?);
    let triggers = Triggers {
        l: u16_le(r, OFF_LT)?,
        r: u16_le(r, OFF_RT)?,
    };
    let sticks = Sticks {
        lx: i16_le(r, OFF_LX)?,
        ly: i16_le(r, OFF_LY)?,
        rx: i16_le(r, OFF_RX)?,
        ry: i16_le(r, OFF_RY)?,
    };
    let l_trackpad = Trackpad {
        x: i16_le(r, OFF_LPAD_X)?,
        y: i16_le(r, OFF_LPAD_Y)?,
        press: i16_le(r, OFF_LPAD_PRESS)?,
    };
    let r_trackpad = Trackpad {
        x: i16_le(r, OFF_RPAD_X)?,
        y: i16_le(r, OFF_RPAD_Y)?,
        press: i16_le(r, OFF_RPAD_PRESS)?,
    };
    let imu = decode_imu(r);

    Some(ControllerInput {
        buttons,
        triggers,
        sticks,
        l_trackpad,
        r_trackpad,
        imu,
    })
}

pub fn parse_input(value: &[u8]) -> Result<InputReport<'_>, ReportError> {
    let id = *value.first().ok_or(ReportError::Empty)?;
    if value.len() < INPUT_MIN_LEN {
        return Err(ReportError::InputTooShort { id, len: value.len() });
    }
    let seq = value.get(OFF_SEQ).copied().unwrap_or(0);
    let input = decode_buttons_triggers_trackpads(value).ok_or(ReportError::InputTooShort { id, len: value.len() })?;

    Ok(InputReport {
        id,
        seq,
        input,
        raw: value,
    })
}

pub fn parse_status(value: &[u8]) -> Result<StatusReport<'_>, ReportError> {
    if value.len() < STATUS_MIN_LEN {
        return Err(ReportError::StatusTooShort { len: value.len() });
    }
    Ok(StatusReport {
        charge_state: value[1],
        battery_pct: value[2],
        raw: value,
    })
}

pub fn parse_report(value: &[u8]) -> Result<Report<'_>, ReportError> {
    let first = value.first().ok_or(ReportError::Empty)?;
    match *first {
        REPORT_ID_INPUT_LEGACY | REPORT_ID_INPUT => parse_input(value).map(Report::Input),
        REPORT_ID_STATUS => parse_status(value).map(Report::Status),
        REPORT_ID_STATUS_EVENT => Ok(Report::StatusEvent(value)),
        id => Ok(Report::Unknown { id, raw: value }),
    }
}