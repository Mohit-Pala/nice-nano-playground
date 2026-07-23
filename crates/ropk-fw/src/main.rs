#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

const LED_PIN_IS_P0: bool = true;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    let mut led = if LED_PIN_IS_P0 {
        Output::new(p.P0_15, Level::Low, OutputDrive::Standard)
    } else {
        Output::new(p.P1_15, Level::Low, OutputDrive::Standard)
    };

    defmt::info!("opk-fw boot: milestone 0 blink");

    loop {
        led.set_high();
        Timer::after(Duration::from_millis(200)).await;
        led.set_low();
        Timer::after(Duration::from_millis(800)).await;
        defmt::info!("tick");
    }
}
