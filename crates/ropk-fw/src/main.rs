#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};


// stole examples from here
// https://github.com/embassy-rs/embassy/blob/main/examples/nrf52840/src/bin/usb_hid_keyboard.rs
use embassy_nrf::{bind_interrupts, peripherals, usb};
use embassy_nrf::usb::Driver;
use embassy_nrf::usb::vbus_detect::HardwareVbusDetect;




// todo:
// research this 
bind_interrupts!(struct Irqs {
    USBD => usb::InterruptHandler<peripherals::USBD>;
    CLOCK_POWER => usb::vbus_detect::InterruptHandler;
});


type Drvr = Driver<'static, peripherals::USBD, HardwareVbusDetect>;

#[embassy_executor::task]
async fn usb_task(mut device: embassy_usb::UsbDevice<'static, Drvr>)
{
    device.run().await;
}    













#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    // let mut led = if LED_PIN_IS_P0 {
    //     Output::new(p.P0_15, Level::Low, OutputDrive::Standard)
    // } else {
    //     Output::new(p.P1_15, Level::Low, OutputDrive::Standard)
    // };

    loop {
        // led.set_high();
        // Timer::after(Duration::from_millis(200)).await;
        // led.set_low();
        // Timer::after(Duration::from_millis(800)).await;
        // defmt::info!("tick");
    }
}
