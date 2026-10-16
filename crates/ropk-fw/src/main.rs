#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_nrf::{
    bind_interrupts, peripherals, usb::Driver, usb::vbus_detect::HardwareVbusDetect,
};
use embassy_time::{Duration, Instant};
use ropk_proto::{
    addr::{DISCOVERY_ADDR, DISCOVERY_CHANNEL},
    frame::{Reply, parse_reply},
    report::Report,
    seq::{Freshness, SeqTracker},
};
use ropk_radio::{sc_radio_config::SteamControllerRadioConfig, sc_radiosetup::ScRadio};
use ropk_usb::
    usb_id_helper::{PuckUsbStruct, build_sc_puck_usb}
;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

#[derive(Default)]
struct Stats {
    rx: u32,
    crc_bad: u32,
    not_ours: u32,
    f1: u32,
    fresh: u32,
    missed: u32,
    frame_err: u32,
}

bind_interrupts!(struct Irqs {
    USBD => embassy_nrf::usb::InterruptHandler<peripherals::USBD>;
    CLOCK_POWER => embassy_nrf::usb::vbus_detect::InterruptHandler;
});

type UsbDriver = Driver<'static, peripherals::USBD, HardwareVbusDetect>;

static RX_BUF: StaticCell<[u8; 100]> = StaticCell::new();
static TX_BUF: StaticCell<[u8; 100]> = StaticCell::new();
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
    let tx_buf: &'static mut [u8; 100] = TX_BUF.init([0; 100]);
    let mut radio = ScRadio::new(embassy_nrf::pac::RADIO, rx_buf, tx_buf);
    let disable_ok = radio.config_radio(&SteamControllerRadioConfig::STEAM_CONTROLLER_RADIO_CONFIG);
    let want = DISCOVERY_ADDR.to_regs();
    let got = radio.regs();
    defmt::info!("disable_ok={} regs={}", disable_ok, got);
    defmt::assert_eq!(got.base0, want.base0); // ibex
    defmt::assert_eq!(got.prefix0, want.prefix0); // 0x08
    defmt::assert_eq!(got.txaddress, 0);
    defmt::assert_eq!(got.rxaddresses, 1);
    defmt::assert_eq!(got.frequency, DISCOVERY_CHANNEL);

    radio.start_sc_radio();
    defmt::info!("radio started");

    let driver = Driver::new(p.USBD, Irqs, HardwareVbusDetect::new(Irqs));
    let usb_device = build_sc_puck_usb(driver, &PUCK_USB_STRUCT);
    spawner.spawn(usb_task(usb_device)).unwrap();

    let mut rx_copy = [0u8; 100];
    let mut stats = Stats::default();
    let mut e1_captured: u8 = 0;
    let mut seq = SeqTracker::new();
    let mut last_stats = Instant::now();

    loop {
        if last_stats.elapsed() >= Duration::from_secs(1) {
            defmt::info!(
                "stats: rx={} crc_bad={} not_ours={} F1={} fresh={} missed={} frame_err={}",
                stats.rx,
                stats.crc_bad,
                stats.not_ours,
                stats.f1,
                stats.fresh,
                stats.missed,
                stats.frame_err
            );
            stats = Stats::default();
            last_stats = Instant::now();
        }

        if let Some(rx) = radio.poll(&mut rx_copy) {
            stats.rx += 1;
            if !rx.crc_ok {
                stats.crc_bad += 1;
                continue;
            }
            match parse_reply(rx.payload) {
                Ok(reply) => {
                    if !reply.is_link_alive() {
                        stats.not_ours += 1;
                    }
                    if let Reply::Input(f) = reply {
                        stats.f1 += 1;
                        for item in f.reports() {
                            match item {
                                Ok(Report::Input(r)) => {
                                    let fr = seq.observe(r.seq);
                                    if let Freshness::Fresh { missed } = fr {
                                        stats.fresh += 1;
                                        stats.missed += missed as u32;
                                    }
                                    defmt::info!("in seq={} {} {}", r.seq, fr, r.input);
                                }
                                Ok(other) => defmt::info!("{}", other),
                                Err(e) => defmt::warn!("report err {}", e),
                            }
                        }
                    } else {
                        if matches!(reply, Reply::NotOurs { rtype: 0xE1 }) && e1_captured < 8 {
                            e1_captured += 1;
                            defmt::info!("Payload  : {=[u8]:02x} s1={=u8:#04x}", rx.payload, rx.s1_pid);
                        }
                        defmt::info!("{}", reply);
                    }
                }
                Err(e) => {
                    stats.frame_err += 1;
                    defmt::warn!("frame err {}", e);
                }
            }
        }
        embassy_futures::yield_now().await;
    }
}