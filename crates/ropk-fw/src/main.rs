#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_nrf::{
    bind_interrupts, peripherals, usb::Driver, usb::vbus_detect::HardwareVbusDetect,
};
use embassy_time::{Duration, Timer};
use embassy_usb::class::hid::{HidReaderWriter, State as HidState};
use ropk_radio::{sc_radio_config::SteamControllerRadioConfig, sc_radiosetup::ScRadio};
use ropk_usb::{
    sc_default_descriptor::PUCK_HID_DESC,
    sc_puck_bond::ScPuckSlot,
    vars::{USB_PROD_ID_STEAM, USB_VENDOR_ID},
};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USBD => embassy_nrf::usb::InterruptHandler<peripherals::USBD>;
    CLOCK_POWER => embassy_nrf::usb::vbus_detect::InterruptHandler;
});

type UsbDriver = Driver<'static, peripherals::USBD, HardwareVbusDetect>;

static RX_BUF: StaticCell<[u8; 100]> = StaticCell::new();

// shit needed for https://docs.embassy.dev/embassy-usb/git/default/struct.Builder.html
static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static CONTROL_BUF: StaticCell<[u8; 128]> = StaticCell::new();

// slots - exapand to 4, this is a fucked way to handle this but fine for teaitng
static SLOT_0_HANDLER: StaticCell<ScPuckSlot> = StaticCell::new();
static SLOT_0_STATE: StaticCell<HidState> = StaticCell::new();
static SLOT_1_HANDLER: StaticCell<ScPuckSlot> = StaticCell::new();
static SLOT_1_STATE: StaticCell<HidState> = StaticCell::new();
static SLOT_2_HANDLER: StaticCell<ScPuckSlot> = StaticCell::new();
static SLOT_2_STATE: StaticCell<HidState> = StaticCell::new();

static SLOT_3_HANDLER: StaticCell<ScPuckSlot> = StaticCell::new();
static SLOT_3_STATE: StaticCell<HidState> = StaticCell::new();


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
    let mut usb_config = embassy_usb::Config::new(USB_VENDOR_ID, USB_PROD_ID_STEAM);

    // from hid cpp
    usb_config.manufacturer = Some("Valve Software");
    usb_config.product = Some("Steam Controller Puck");
    usb_config.device_class = 0x00;
    usb_config.device_release = 0x0211;
    // from identity - use some hardcoded this shit for now, replace with nrf silicon id later 
    usb_config.serial_number = Some("FXB9960200000");


    let mut builder = embassy_usb::Builder::new(
        driver,
        usb_config,
        CONFIG_DESCRIPTOR.init([0; 256]),
        BOS_DESCRIPTOR.init([0; 256]),
        &mut [],
        CONTROL_BUF.init([0; 128]),
    );

    let slot_0 = SLOT_0_HANDLER.init(ScPuckSlot::new());
    let slot_0_state = SLOT_0_STATE.init(HidState::new());

    let hid_config = embassy_usb::class::hid::Config {
        report_descriptor: PUCK_HID_DESC,
        request_handler: Some(slot_0),
        poll_ms: 1,
        max_packet_size: 64,
        hid_boot_protocol: embassy_usb::class::hid::HidBootProtocol::None,
        hid_subclass: embassy_usb::class::hid::HidSubclass::No,
    };

    let _hid_slot_0 = HidReaderWriter::<_, 64, 64>::new(&mut builder, slot_0_state, hid_config);

    let usb_device = builder.build();
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
