use embassy_nrf::pac::radio::regs::{
    Crccnf, Crcinit, Crcpoly, Pcnf0, Pcnf1, Rxaddresses, Txaddress,
};
use embassy_nrf::pac::radio::vals::Mode;

pub struct SteamControllerRadioConfig {
    pub base: [u8; 4],
    pub prefix: u8,
    pub frequency: u8,
    pub mode: Mode,
    pub tx_addr: Txaddress,
    pub rx_addr: Rxaddresses,
    pub pcnf0: Pcnf0,
    pub pcnf1: Pcnf1,
    pub crccnf: Crccnf,
    pub crcinit: Crcinit,
    pub crcpoly: Crcpoly,
}

// configs for the steam controller
// steam controllers use hex ibex as they base address and 0x10 as prefix
// values from radio.cpp, protocol.md says 0x01040040, radio cpp has a comment stating it needs to be >= 66
pub const STEAM_CONTROLLER_RADIO_CONFIG: SteamControllerRadioConfig = SteamControllerRadioConfig {
    base: *b"ibex",
    prefix: 0x10,
    frequency: 2,
    mode: Mode::BLE_2MBIT,
    tx_addr: Txaddress(0),
    rx_addr: Rxaddresses(0),
    pcnf0: Pcnf0(0x0003_0008),
    pcnf1: Pcnf1(0x0104_0060),
    crccnf: Crccnf(2),
    crcpoly: Crcpoly(0x11021),
    crcinit: Crcinit(0xFFFF),
};
