use crate::vars::{ATTR83, ATTR83_LEN, Cmd, PROD_ID_CTRL};
use embassy_usb::class::hid::{ReportId, RequestHandler};
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

        let command = data[0];
        let len = if data.len() > 1 { data[1] } else { 0 };
        let payload = if data.len() > 2 { &data[2..] } else { &[] };

        defmt::info!("cmd: {=u8:#04X}, len: {=u8}", command, len);

        self.resp.fill(0);
        self.resp_len = 63;

        // https://doc.rust-lang.org/book/ch06-02-match.html
        // https://doc.rust-lang.org/book/ch19-03-pattern-syntax.html
        // rabbit hole: match is really powerful

        match Cmd::try_from(command) {
            Ok(Cmd::GetAttributes) => {
                self.resp[0] = Cmd::GetAttributes as u8;
                self.resp[1] = ATTR83_LEN as u8;

                let end_idx = 2 + ATTR83_LEN;
                self.resp[2..end_idx].copy_from_slice(&ATTR83);

                if let ReportId::Feature(1) = id {
                    self.resp[3] = PROD_ID_CTRL;
                }
            }

            Ok(Cmd::GetStrings) => {
                self.resp[0] = Cmd::GetStrings as u8;
                self.resp[1] = 0x14;
                let idx = if !payload.is_empty() { payload[0] } else { 1 };
                self.resp[2] = idx;
                //
                if (idx == 0 || idx == 1 || idx == 4) && self.has_bond {
                    self.resp[3..19].copy_from_slice(&self.bond[8..24]);
                } else {
                    self.resp[3] = b'N';
                    self.resp[4] = b'A';
                }
            }

            Ok(Cmd::GetConnectionState) => {
                self.resp[0] = Cmd::GetConnectionState as u8;
                self.resp[1] = 0x01;
                self.resp[2] = 0x01;
            }

            Ok(Cmd::SetPairingMode) => {
                self.resp[0] = Cmd::SetPairingMode as u8;
                self.resp[1] = 0x00;
            }

            Ok(Cmd::WriteBond) => {
                if payload.len() >= 24 {
                    let is_empty = payload[..24].iter().all(|&b| b == 0);
                    if is_empty {
                        self.has_bond = false;
                        self.bond.fill(0);
                    } else {
                        self.has_bond = true;
                        self.bond.copy_from_slice(&payload[..24]);
                    }
                }
                self.resp[0] = Cmd::WriteBond as u8;
                self.resp[1] = 0x00;
            }

            Ok(Cmd::ReadBond) => {
                self.resp[0] = Cmd::ReadBond as u8;
                self.resp[1] = 0x18;
                if self.has_bond {
                    self.resp[2..26].copy_from_slice(&self.bond);
                }
            }

            Ok(Cmd::SetSettingsValues) => {
                self.resp[0] = Cmd::SetSettingsValues as u8;
                self.resp[1] = 0x00;
            }

            Ok(Cmd::GetSettingsValues) => {
                self.resp[0] = Cmd::GetSettingsValues as u8;
                self.resp[1] = 0x03;
                self.resp[2] = if !payload.is_empty() { payload[0] } else { 0 };
            }

            Ok(Cmd::GetSettingByPath) => {
                self.resp[0] = Cmd::GetSettingByPath as u8;
                if let Ok(path_str) = core::str::from_utf8(payload) {
                    if path_str.starts_with("esb/bond") && self.has_bond {
                        self.resp[1] = 0x18;
                        self.resp[2..26].copy_from_slice(&self.bond);
                    } else if path_str.starts_with("user/wireless_transport") {
                        self.resp[1] = 0x01;
                        self.resp[2] = 0x02;
                    } else {
                        self.resp[1] = 0x00;
                    }
                } else {
                    self.resp[1] = 0x00;
                }
            }

            Err(unhandled_byte) => {
                self.resp[0] = unhandled_byte;
                self.resp[1] = len;
                let cpy_len = payload.len().min(60);
                self.resp[2..2 + cpy_len].copy_from_slice(&payload[..cpy_len]);
            }
        }

        OutResponse::Accepted
    }
}
