#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};



// helper funcs - move to a serivces file
 pub fn rf_bitrev8(val: u8) -> u8 {
    val.reverse_bits()
}



#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    // dont use this radio, cant get low levelaccesss to it
    // let radio = p.RADIO;

    // use the usntable pac radio instead since i nee low leberl control
    let radio = embassy_nrf::pac::RADIO;
    defmt::info!("pac radio setup");


    // docs -> steam controllers use hex ibex as they base address and 0x10 as prefix
    // todo: move these vals to a dedicated file under radio
    let sc_base_address: [u8; 4] = *b"ibex"; 
    let sc_prefix: u8 = 0x10;

    // next few lines are doing what the rfSetAddr did
    let rev_base: [u8; 4] = sc_base_address.map(rf_bitrev8); // bit reverse every byte and return as an array
    let rev_prefix: u8 = rf_bitrev8(sc_prefix); // same for prefix
    let base0_val = u32::from_be_bytes(rev_base[0..4].try_into().unwrap());
    

}
