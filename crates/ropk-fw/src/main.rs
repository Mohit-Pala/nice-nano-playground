#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_nrf::pac::radio::regs::{
    Crccnf, Crcinit, Crcpoly, Pcnf0, Pcnf1, Prefix0, Rxaddresses, Txaddress,
};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

// helper funcs - move to a serivces file
pub fn rf_bitrev8(val: u8) -> u8 {
    val.reverse_bits()
}



#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut config = embassy_nrf::config::Config::default();
    config.hfclk_source = embassy_nrf::config::HfclkSource::ExternalXtal;
    let _p = embassy_nrf::init(config);
    // let _p = embassy_nrf::init(Default::default());

    // dont use this radio, cant get low levelaccesss to it
    // let radio = p.RADIO;

    // use the usntable pac radio instead since i nee low leberl control
    let radio = embassy_nrf::pac::RADIO;
    defmt::info!("pac radio setup");

    // docs -> steam controllers use hex ibex as they base address and 0x10 as prefix
    // values from radio.cpp, protocol.md says 0x01040040, radio cpp has a comment stating it needs to be >= 66
    // todo: move these vals to a dedicated file under radio

    let sc_base_address: [u8; 4] = *b"ibex";
    let sc_prefix: u8 = 0x10;
    let sc_txaddr: Txaddress = Txaddress(0);
    let sc_rxaddr: Rxaddresses = Rxaddresses(1);
    let sc_pcnf0: Pcnf0 = Pcnf0(0x0003_0008);
    let sc_pcnf1: Pcnf1 = Pcnf1(0x0104_0060);
    let sc_crccnf: Crccnf = Crccnf(2);
    let sc_crcpoly: Crcpoly = Crcpoly(0x11021);
    let sc_crcinit: Crcinit = Crcinit(0xFFFF);

    // next few lines are doing what the rfSetAddr did
    let rev_base: [u8; 4] = sc_base_address.map(rf_bitrev8); // bit reverse every byte and return as an array
    let rev_prefix: Prefix0 = Prefix0(rf_bitrev8(sc_prefix) as u32); // same for prefix, needs to be prefix 0 type
    let base0_val = u32::from_be_bytes(rev_base[0..4].try_into().unwrap());
    radio.base0().write_value(base0_val);
    radio.prefix0().write_value(rev_prefix);
    radio.txaddress().write_value(sc_txaddr);
    radio.rxaddresses().write_value(sc_rxaddr);

    // from rfconfig method
    radio.pcnf0().write_value(sc_pcnf0);
    radio.pcnf1().write_value(sc_pcnf1);
    radio.crccnf().write_value(sc_crccnf);
    radio.crcpoly().write_value(sc_crcpoly);
    radio.crcinit().write_value(sc_crcinit);

    let rx_buf: [u8; 100] = [0; 100];

    // radio.mode().write_value(BLE_2MBIT);
    radio.mode().write(|w| w.set_mode(embassy_nrf::pac::radio::vals::Mode::BLE_2MBIT));
    radio.frequency().write(|w| w.set_frequency(2));
    radio.packetptr().write_value(rx_buf.as_ptr() as u32);


    defmt::info!("written to radio");

    // shorts
    radio.shorts().write(|w| {
        w.set_ready_start(true);
        w.set_end_start(true);
        w.set_address_rssistart(true);
        w.set_disabled_rssistop(true);
    });
    // Enable RADIO in RX mode 
    radio.tasks_rxen().write_value(true as u32);
    defmt::info!("radio enabled in RX mode");

    loop {
        if radio.events_end().read() != 0 {
            radio.events_end().write_value(0);
            // defmt::info!("radio end");
            let crc_ok = (radio.crcstatus().read().0 & 1) != 0;
            let raw_len = rx_buf[0] as usize;
            let s1_pid = rx_buf[1];
            
            let payload_len = if raw_len > 0 { raw_len - 1 } else { 0 };
            let valid_len = payload_len.min(rx_buf.len().saturating_sub(2));  // min of payload len vs 100 - 2
            let payload = &rx_buf[2..2 + valid_len]; // array slice from idx 2 to 2 + 17? 

            defmt::info!("Log start");
            defmt::info!("CRC OK   : {}", crc_ok);
            defmt::info!("S1/PID   : 0x{:02x}", s1_pid);
            defmt::info!("Length   : {} bytes", payload_len);
            defmt::info!("Payload  : {=[u8]:02x}", payload);
            defmt::info!("Log end");
        }
        Timer::after(Duration::from_millis(1)).await;
    }
}
