use embassy_usb::class::hid::{RequestHandler, ReportId};
use embassy_usb::control::OutResponse;

// from bonds.h
// 8 + 16 = 24
pub struct ScPuckSlot {
    bond: [u8; 24], // james
    has_bond: bool, // basically used bool
    resp: [u8; 63],
    resp_len: usize,
}

impl ScPuckSlot {
    pub const fn new() -> Self {
        Self {
            bond: [0; 24],
            has_bond: false,
            resp: [0; 63],
            resp_len: 0,
        }
    }
}

impl RequestHandler for ScPuckSlot {
    fn get_report(&mut self, id: ReportId, buf: &mut [u8]) -> Option<usize> {
        // copies the first len bytes to resp
        let len = self.resp_len.min(buf.len());
        buf[..len].copy_from_slice(&self.resp[..len]);
        Some(len)
    }
    
    fn set_report(&mut self, id: ReportId, data: &[u8]) -> OutResponse {
        // cleep time
    }
}