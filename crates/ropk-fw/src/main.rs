#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
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
    let mut radio = Radio::new(embassy_nrf::pac::RADIO, rx_buf);

    loop {
        Timer::after(Duration::from_millis(1)).await;
    }
}
