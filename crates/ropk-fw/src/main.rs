#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};


// stole examples from here
// https://github.com/embassy-rs/embassy/blob/main/examples/nrf52840/src/bin/usb_hid_keyboard.rs
use embassy_nrf::{bind_interrupts, peripherals, usb};
use embassy_nrf::usb::Driver;
use embassy_nrf::usb::vbus_detect::HardwareVbusDetect;
use embassy_usb::class::hid::{Config as HidConfig, HidReaderWriter, State};
use embassy_usb::Config;
use static_cell::StaticCell;
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};


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










// func, type test every x seconds with y milli-seconds bw each letter
//
async fn type_test() {
    Timer::after(Duration::from_secs(5)).await;
    defmt::info!("----xxxx----");


    // regarding keycodes
    // https://www.reddit.com/r/embedded/comments/lz7e14/hid_keyboard_keycodes/
    // https://gist.github.com/MightyPork/6da26e382a7ad91b5496ee55fdc73db2

    // fuckass task
    defmt ::info!("typing t");
    defmt::info!("e");
    defmt::info!("s");
    defmt::info!("t");

    defmt::info!("----xxxx----");
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // most of this is stolen from https://github.com/embassy-rs/embassy/blob/main/examples/nrf52840/src/bin/usb_hid_keyboard.rs
    let p = embassy_nrf::init(Default::default());
    
     // Create the driver
    let vbus_detect = HardwareVbusDetect::new(Irqs);
    let driver = Driver::new(p.USBD, Irqs, vbus_detect);

     // Create embassy-usb Config
    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("Embassy");
    config.product = Some("HID keyboard example");
    config.serial_number = Some("12345678");

    // example code explains why they need to be made static
    static CONFIG_DESC: StaticCell<[u8; 256]> = StaticCell::new();
    static BOS_DESC: StaticCell<[u8; 256]> = StaticCell::new();
    static MSOS_DESC: StaticCell<[u8; 256]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

    let mut builder = embassy_usb::Builder::new(
        driver,
        config,
        CONFIG_DESC.init([0; 256]),
        BOS_DESC.init([0; 256]),
        MSOS_DESC.init([0; 256]),
        CONTROL_BUF.init([0; 64]),
    );

    static HID_STATE: StaticCell<State> = StaticCell::new();
    let hid_config = HidConfig {
        report_descriptor: KeyboardReport::desc(),
        request_handler: None,
        poll_ms: 10,
        max_packet_size: 8,
        hid_subclass: embassy_usb::class::hid::HidSubclass::Boot,
        hid_boot_protocol: embassy_usb::class::hid::HidBootProtocol::Keyboard,
    };

    let hid = HidReaderWriter::<_, 1, 8>::new(
        &mut builder,
        HID_STATE.init(State::new()),
        hid_config,
    );
    let (_reader, mut writer) = hid.split();

    let usb_device = builder.build();
    spawner.spawn(usb_task(usb_device)).unwrap();

    loop {
        Timer::after(Duration::from_secs(5)).await;
        
        // move these to a func
        defmt::info!("Sending 'o'");
        let report = KeyboardReport { modifier: 0, reserved: 0, leds: 0, keycodes: [0x12, 0, 0, 0, 0, 0] };
        let _ = writer.write_serialize(&report).await;
        Timer::after(Duration::from_millis(30)).await;
        let report = KeyboardReport { modifier: 0, reserved: 0, leds: 0, keycodes: [0; 6] };
        let _ = writer.write_serialize(&report).await;
        
        Timer::after(Duration::from_millis(50)).await;

        defmt::info!("Sending 'k'");
        let report = KeyboardReport { modifier: 0, reserved: 0, leds: 0, keycodes: [0x0E, 0, 0, 0, 0, 0] };
        let _ = writer.write_serialize(&report).await;
        Timer::after(Duration::from_millis(30)).await;
        let report = KeyboardReport { modifier: 0, reserved: 0, leds: 0, keycodes: [0; 6] };
        let _ = writer.write_serialize(&report).await;
    }
}
