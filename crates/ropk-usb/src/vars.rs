// rename this file

// USB Product and Vendor IDs
// from  puck HID cpp and protocol md
pub const USB_VENDOR_ID: u16 = 0x28DE;
pub const USB_PROD_ID_STEAM: u16 = 0x1142;

// FROM PIUCK HID CPP AND IDENTITY CPP
// 1304 IS puck and 02 is conteroller
pub const PROD_ID_PUCK: u8 = 0x04; // 0x1304
pub const PROD_ID_CTRL: u8 = 0x02; // 0x1302

// from identitry cpp
pub const ATTR83: [u8; 25] = [
    0x01, 0x04, 0x13, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
    0x00, 0x0A, 0xF2, 0xF9, 0xD2, 0x68, 0x04, 0x53, 0xD0,
    0x18, 0x6A, 0x09, 0x47, 0x00, 0x00, 0x00,
];
pub const ATTR83_LEN: usize = ATTR83.len();


// steam feat codes - refer to sec 3.2 of protocol md
// breaking this into an enum - to make it easier to read 
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmd {
    GetAttributes       = 0x83,
    GetStrings          = 0xAE,
    GetConnectionState  = 0xB4,
    SetPairingMode      = 0xAD,
    WriteBond           = 0xA2,
    ReadBond            = 0xA3,
    SetSettingsValues   = 0x87,
    GetSettingsValues   = 0x89,
    GetSettingByPath    = 0xED,
}

impl TryFrom<u8> for Cmd {
    type Error = u8;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x83 => Ok(Cmd::GetAttributes),
            0xAE => Ok(Cmd::GetStrings),
            0xB4 => Ok(Cmd::GetConnectionState),
            0xAD => Ok(Cmd::SetPairingMode),
            0xA2 => Ok(Cmd::WriteBond),
            0xA3 => Ok(Cmd::ReadBond),
            0x87 => Ok(Cmd::SetSettingsValues),
            0x89 => Ok(Cmd::GetSettingsValues),
            0xED => Ok(Cmd::GetSettingByPath),
            unhandled => Err(unhandled),
        }
    }
}