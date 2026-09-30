use embassy_nrf::pac::radio::regs::{Crccnf, Crcinit, Crcpoly, Pcnf0, Pcnf1};
use embassy_nrf::pac::radio::vals::{Mode, Txpower};
use ropk_proto::addr::{DISCOVERY_ADDR, DISCOVERY_CHANNEL, RadioAddr};

pub struct SteamControllerRadioConfig {
    pub addr: RadioAddr,
    pub frequency: u8,
    pub mode: Mode,
    pub tx_power: Txpower,
    pub pcnf0: Pcnf0,
    pub pcnf1: Pcnf1,
    pub crccnf: Crccnf,
    pub crcinit: Crcinit,
    pub crcpoly: Crcpoly,
}

impl SteamControllerRadioConfig {
    // configs for the steam controller
    // steam controllers use hex ibex as they base address and 0x10 as prefix
    // values from radio.cpp, protocol.md says 0x01040040, radio cpp has a comment stating it needs to be >= 66
    pub const STEAM_CONTROLLER_RADIO_CONFIG: SteamControllerRadioConfig = SteamControllerRadioConfig {
        addr: DISCOVERY_ADDR,
        frequency: DISCOVERY_CHANNEL,
        mode: Mode::BLE_2MBIT,
        // max. the pro micro's pcb trace antenna reads ~20dB below a real puck, and a poll the
        // controller can't hear is a missed reply
        tx_power: Txpower::POS8_DBM,
        pcnf0: Pcnf0(0x0003_0008),
        pcnf1: Pcnf1(0x0104_0060),
        crccnf: Crccnf(2),
        crcpoly: Crcpoly(0x11021),
        crcinit: Crcinit(0xFFFF),
    };
}