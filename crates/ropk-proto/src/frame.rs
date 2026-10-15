use core::iter::FusedIterator;

use crate::{report::{Report, ReportError, parse_report}, tlv::{TAG_REPORT, TlvError, TlvIter}};

pub const MAX_PAYLOAD_LEN: usize = 96; // rf_link.cpp's max payload len - line 3 09
pub const REPLY_INPUT: u8 = 0xF1;
pub const REPLY_DISCONNECT: u8 = 0xF2; // disconnect / poweroff around line 371 and 376
pub const REPLY_STATUS: u8 = 0xF3; // Status/version rf_link.cpp -399
pub const REPLY_OURS_MIN: u8 = 0xF0; // rf_link.cpp- 318
pub const OFF_F3_VERSION: usize = 4;
pub const F3_MIN_LEN: usize = 5;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct InputFrame<'a> {
    body: &'a [u8],
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FrameError {
    Empty,
    TooLong { len: usize },
    StatusTooShort { len: usize },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReportItemError {
    Tlv(TlvError),
    Report(ReportError),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reply<'a> {
    Input(InputFrame<'a>),
    Disconnect,
    Status { version: u8, raw: &'a [u8] },
    OtherReply { rtype: u8, raw: &'a [u8] }, // f-type - refer to rflink's comments
    NotOurs { rtype: u8 }, // e-type: another puck's beacon or poll
}


pub fn parse_reply(payload: &[u8]) -> Result<Reply<'_>, FrameError> {
    if payload.is_empty() {
        return Err(FrameError::Empty);
    }

    if payload.len() > MAX_PAYLOAD_LEN {
        return Err(FrameError::TooLong { len: payload.len() });
    }

    // todo, check if we extract rtyope anywere else if so make this a helper
    let Some((&rtype, body)) = payload.split_first() else {
        return Err(FrameError::Empty);
    };

    if rtype < REPLY_OURS_MIN {
        return Ok(Reply::NotOurs { rtype });
    }

    match rtype {
        REPLY_INPUT => Ok(Reply::Input(InputFrame { body })),
        REPLY_DISCONNECT => Ok(Reply::Disconnect),
        REPLY_STATUS => {
            if payload.len() < F3_MIN_LEN {
                Err(FrameError::StatusTooShort { len: payload.len() })
            } else {
                Ok(Reply::Status {
                    version: payload[OFF_F3_VERSION],
                    raw: payload,
                })
            }
        }
        _ => Ok(Reply::OtherReply {
            rtype,
            raw: payload,
        }),
    }
}


pub struct Reports<'a> {
    iter: TlvIter<'a>,
}

// tlviter contails s bunch of shit like statuses and other tags, we only need reports here
impl<'a> Iterator for Reports<'a> {
    type Item = Result<Report<'a>, ReportItemError>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter.next()? {
                Ok(tlv) => {
                    if tlv.tag != TAG_REPORT {
                        continue;
                    }
                    return Some(parse_report(tlv.value).map_err(ReportItemError::Report));
                }
                Err(e) => {
                    return Some(Err(ReportItemError::Tlv(e)));
                }
            }
        }
    }
}

impl<'a> FusedIterator for Reports<'a> {}

impl<'a> InputFrame<'a> {
    pub fn tlvs(&self) -> TlvIter<'a> {
        TlvIter::new(self.body)
    }

    pub fn reports(&self) -> Reports<'a> {
        Reports {
            iter: self.tlvs(),
        }
    }
}

impl Reply<'_> {
    pub fn is_link_alive(&self) -> bool {
        !matches!(self, Reply::NotOurs { .. })
    }
}