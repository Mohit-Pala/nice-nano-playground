pub mod sc_radio_config;
pub mod sc_radiosetup;
pub mod sc_radio_data;



pub fn bitrev8(val: u8) -> u8 {
    return val.reverse_bits();
}