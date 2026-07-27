use embassy_nrf::pac::radio::{Radio, regs::Prefix0};

use crate::{bitrev8, sc_radio_config::SteamControllerRadioConfig, sc_radio_data::ScRadioData};

pub struct ScRadio {
    sc_radio: Radio,
    rx_buf: &'static mut [u8] 
}

impl ScRadio {
    pub fn new(sc_radio: Radio, rx_buf: &'static mut [u8]) -> Self {
        Self { sc_radio, rx_buf }
    }

    pub fn config_radio(&mut self, sc_config: &SteamControllerRadioConfig) {
        let rev_base: [u8; 4] = sc_config.base.map(bitrev8);
        let base0: u32 = u32::from_be_bytes(rev_base);
        let prefix0: Prefix0 = Prefix0(bitrev8(sc_config.prefix) as u32);
        

        // next few lines are doing what the rfSetAddr did
        self.sc_radio.base0().write_value(base0);
        self.sc_radio.prefix0().write_value(prefix0);
        self.sc_radio.txaddress().write_value(sc_config.tx_addr);
        self.sc_radio.rxaddresses().write_value(sc_config.rx_addr);

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
        // i dont fuckn know why but dont use ; for default return path
        Some(ScRadioData::from_buf(self.rx_buf, crc_ok))
    }
}

