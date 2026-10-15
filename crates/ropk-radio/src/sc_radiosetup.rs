use embassy_nrf::pac::radio::{Radio, regs::{Prefix0, Rxaddresses, Txaddress}};
use embassy_time::{Duration, Instant};
use ropk_proto::addr::RadioAddr;

use crate::{sc_radio_config::SteamControllerRadioConfig, sc_radio_data::ScRadioData};

pub const TX_LOGICAL_ADDR: Txaddress = Txaddress(0);
pub const RX_ADDRESSES_ADDR0: Rxaddresses = Rxaddresses(1 << 0);

#[derive(defmt::Format, Debug, PartialEq, Eq)]
pub struct RadioRegs {
    pub base0: u32,
    pub prefix0: u32,
    pub txaddress: u32,
    pub rxaddresses: u32,
    pub frequency: u8,
    pub pcnf0: u32,
    pub pcnf1: u32,
}

pub struct ScRadio {
    sc_radio: Radio,
    rx_buf: &'static mut [u8],
    tx_buf: &'static mut [u8]
}

impl ScRadio {



    // helper methods - move this to a new file

    // from radio h
    pub fn wait_disabled(&self) -> bool {
        let start = Instant::now();
        let timeout = Duration::from_micros(3000);

        while self.sc_radio.events_disabled().read() == 0 {
            if start.elapsed() >= timeout {
                return false;
            }
        }
        return true;
    }

    fn disable(&mut self) -> bool {
        self.sc_radio.events_disabled().write_value(0);
        self.sc_radio.tasks_disable().write_value(1);
        let ok = self.wait_disabled();
        self.sc_radio.events_disabled().write_value(0);
        ok
    }

    fn write_addr_regs(&mut self, addr: RadioAddr) {
        let regs = addr.to_regs();
        self.sc_radio.base0().write_value(regs.base0);
        self.sc_radio.prefix0().write_value(Prefix0(regs.prefix0));
        self.sc_radio.txaddress().write_value(TX_LOGICAL_ADDR);
        self.sc_radio.rxaddresses().write_value(RX_ADDRESSES_ADDR0);
    }

    pub fn set_channel(&mut self, frequency: u8) -> bool {
        let ok = self.disable();
        if ok {
            self.sc_radio.frequency().write(|w| w.set_frequency(frequency));
        }
        ok
    }


    pub fn set_addr(&mut self, addr: RadioAddr) -> bool {
        let ok = self.disable();
        if ok {
            self.write_addr_regs(addr);
        }
        ok
    }

    pub fn regs(&self) -> RadioRegs {
        RadioRegs {
            base0: self.sc_radio.base0().read(),
            prefix0: self.sc_radio.prefix0().read().0,
            txaddress: self.sc_radio.txaddress().read().0,
            rxaddresses: self.sc_radio.rxaddresses().read().0,
            frequency: self.sc_radio.frequency().read().frequency(),
            pcnf0: self.sc_radio.pcnf0().read().0,
            pcnf1: self.sc_radio.pcnf1().read().0,
        }
    }

    pub fn new(sc_radio: Radio, rx_buf: &'static mut [u8], tx_buf: &'static mut [u8]) -> Self {
        Self { sc_radio, rx_buf, tx_buf }
    }

    pub fn config_radio(&mut self, sc_config: &SteamControllerRadioConfig) -> bool {
        let ok = self.disable();
        if !ok {
            defmt::warn!("config_radio: disable() timed out");
        }

        self.write_addr_regs(sc_config.addr);

        // from rfconfig method
        self.sc_radio.pcnf0().write_value(sc_config.pcnf0);
        self.sc_radio.pcnf1().write_value(sc_config.pcnf1);
        self.sc_radio.crccnf().write_value(sc_config.crccnf);
        self.sc_radio.crcpoly().write_value(sc_config.crcpoly);
        self.sc_radio.crcinit().write_value(sc_config.crcinit);
        
        self.sc_radio.mode().write(|w| w.set_mode(sc_config.mode));
        self.sc_radio.frequency().write(|w| w.set_frequency(sc_config.frequency));
        self.sc_radio.packetptr().write_value(self.rx_buf.as_ptr() as u32);

        // shorts
        self.sc_radio.shorts().write(|w| {
            w.set_ready_start(true);
            w.set_end_start(true);
            w.set_address_rssistart(true);
            w.set_disabled_rssistop(true);
        });

        ok
    }   

    pub fn start_sc_radio(&self) {
        self.sc_radio.tasks_rxen().write_value(1);
    }

    pub fn poll(&self) -> Option<ScRadioData<'_>> {
        if self.sc_radio.events_end().read() == 0 {
            return None;
        }
        self.sc_radio.events_end().write_value(0);
        let crc_ok = (self.sc_radio.crcstatus().read().0 & 1) != 0;
        Some(ScRadioData::from_buf(self.rx_buf, crc_ok))
    }

    // from rf link cpp line 194
    pub fn transmit(&mut self, frame: &[u8]) -> bool {
        self.tx_buf[..frame.len()].copy_from_slice(&frame);
        self.sc_radio.packetptr().write_value(self.tx_buf.as_ptr() as u32);
        self.sc_radio.shorts().write(|w| {
            w.set_ready_start(true);
            w.set_end_disable(true);
        });
        self.sc_radio.events_disabled().write_value(0);
        self.sc_radio.tasks_txen().write_value(1);
        let ok = self.wait_disabled();
        self.sc_radio.events_disabled().write_value(0);
        
        // note: c code dont have any return type
        return ok;
    }


}

