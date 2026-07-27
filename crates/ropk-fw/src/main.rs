#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use ropk_radio::{sc_radio_config::SteamControllerRadioConfig, sc_radiosetup::ScRadio};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

static RX_BUF: StaticCell<[u8; 100]> = StaticCell::new();

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // need the crystal clock, not the internal clock, wihtout this the radio doesnt seem to be picking shit up
    let mut config = embassy_nrf::config::Config::default();
    config.hfclk_source = embassy_nrf::config::HfclkSource::ExternalXtal;
    let _p = embassy_nrf::init(config);
    // let _p = embassy_nrf::init(Default::default());

    // dont use this radio, cant get low level access to it
    // let radio = p.RADIO;
    // use the usntable pac radio instead since i nee low leberl control
    let rx_buf: &'static mut [u8; 100] = RX_BUF.init([0; 100]);
    let mut radio = ScRadio::new(embassy_nrf::pac::RADIO, rx_buf);
    radio.config_radio(&SteamControllerRadioConfig::STEAM_CONTROLLER_RADIO_CONFIG);
    radio.start_sc_radio();
    defmt::info!("radio started");

    loop {
        if let Some(sc_radio_data) = radio.poll() {
            defmt::info!("Log start");
            defmt::info!("CRC OK   : {}", sc_radio_data.crc_ok);
            defmt::info!("S1/PID   : 0x{:02x}", sc_radio_data.s1_pid);
            defmt::info!("Length   : {} bytes", sc_radio_data.payload.len());
            defmt::info!("Payload  : {=[u8]:02x}", sc_radio_data.payload);
            defmt::info!("Log end");
        }
        Timer::after(Duration::from_millis(1)).await;
    }
}
