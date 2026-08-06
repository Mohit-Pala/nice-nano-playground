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
        if data.is_empty() {
            return OutResponse::Accepted;
        }

        let cmd = data[0];
        let len = if data.len() > 1 { data[1] } else { 0 };
        let payload = if data.len() > 2 { &data[2..] } else { &[] };

        defmt::info!("cmd: {=u8:#04X}, len: {=u8}", cmd, len);
        
        self.resp.fill(0);
        self.resp_len = 63;

        // https://doc.rust-lang.org/book/ch06-02-match.html
        // https://doc.rust-lang.org/book/ch19-03-pattern-syntax.html
        // rabbithole: match is really powerful
        match cmd {
            // case 0x83
            0x83 => {
                self.resp[0] = 0x83;
                self.resp[1] = 0x25;
                // let OxAttr83: [u8; 25] = [

                // ]
            }

            // fuckass read squiggly
            _ => {

            }
        }

        OutResponse::Accepted
    }
}