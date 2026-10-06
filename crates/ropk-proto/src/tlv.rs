use core::iter::FusedIterator;

// protocol md 7.3
pub const TAG_STATUS: u8 = 0x2;
pub const TAG_BLOB: u8 = 0x4;
pub const TAG_REPORT: u8 = 0x6;
pub const HEADER_LEN: usize = 0x2; // [len][tag]

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Tlv<'a> {
    pub tag: u8,
    pub value: &'a [u8],
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TlvError {
    HeaderTruncated { at: usize },
    ValueTruncated { at: usize, len: u8, remaining: usize },
}

pub struct TlvIter<'a> {
    buf: &'a [u8],
    pos: usize,
    done: bool,
}

impl<'a> TlvIter<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self {
            buf,
            pos: 0,
            done: false,
        }
    }
}

impl<'a> Iterator for TlvIter<'a> {
    type Item = Result<Tlv<'a>, TlvError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let buf = self.buf;
        let pos = self.pos;

        if pos == buf.len() {
            self.done = true;
            return None;
        }

        let remaining = buf.len() - pos;

        // only 1 byte left
        if remaining < HEADER_LEN {
            self.done = true;
            return Some(Err(TlvError::HeaderTruncated { at: pos }));
        }

        let len = buf[pos];
        let tag = buf[pos + 1];

        // rflink len == 0 
        if len == 0 {
            self.done = true;
            return None;
        }

        let value_remaining = remaining - HEADER_LEN;
        let len_usize = len as usize;

        
        if value_remaining < len_usize {
            self.done = true;
            return Some(Err(TlvError::ValueTruncated {
                at: pos,
                len,
                remaining: value_remaining,
            }));
        }

        let val_start = pos + HEADER_LEN;
        let val_end = val_start + len_usize;
        let value = &buf[val_start..val_end];
        self.pos = val_end;

        Some(Ok(Tlv { tag, value }))
    }
}

impl<'a> FusedIterator for TlvIter<'a> {}