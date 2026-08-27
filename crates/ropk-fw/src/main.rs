#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_nrf::{
    bind_interrupts, peripherals, usb::Driver, usb::vbus_detect::HardwareVbusDetect,
};
use embassy_time::{Duration, Timer};
use ropk_radio::{sc_radio_config::SteamControllerRadioConfig, sc_radiosetup::ScRadio};
use ropk_usb::
    usb_id_helper::{PuckUsbStruct, build_sc_puck_usb}
;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USBD => embassy_nrf::usb::InterruptHandler<peripherals::USBD>;
    CLOCK_POWER => embassy_nrf::usb::vbus_detect::InterruptHandler;
});

type UsbDriver = Driver<'static, peripherals::USBD, HardwareVbusDetect>;

static RX_BUF: StaticCell<[u8; 100]> = StaticCell::new();
static PUCK_USB_STRUCT: PuckUsbStruct = PuckUsbStruct::new();


#[embassy_executor::task]
async fn usb_task(mut usb: embassy_usb::UsbDevice<'static, UsbDriver>) {
    usb.run().await;
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {    
    // need the crystal clock, not the internal clock, wihtout this the radio doesnt seem to be picking shit up
    let mut config = embassy_nrf::config::Config::default();
    config.hfclk_source = embassy_nrf::config::HfclkSource::ExternalXtal;
    let p = embassy_nrf::init(config);
    // let _p = embassy_nrf::init(Default::default());

    // dont use this radio, cant get low level access to it
    // let radio = p.RADIO;
    // use the usntable pac radio instead since i nee low leberl control
    let rx_buf: &'static mut [u8; 100] = RX_BUF.init([0; 100]);
    let mut radio = ScRadio::new(embassy_nrf::pac::RADIO, rx_buf);
    radio.config_radio(&SteamControllerRadioConfig::STEAM_CONTROLLER_RADIO_CONFIG);
    radio.start_sc_radio();
    defmt::info!("radio started");

    let driver = Driver::new(p.USBD, Irqs, HardwareVbusDetect::new(Irqs));
    let usb_device = build_sc_puck_usb(driver, &PUCK_USB_STRUCT);
    spawner.spawn(usb_task(usb_device)).unwrap();

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
