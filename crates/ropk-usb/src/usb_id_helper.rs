pub struct UsbDescLens {
    conf_desc_len: u16,
    bos_desc_len: u16,
    ctrl_buf_len: u16
}

impl UsbDescLens {
    const fn new() -> Self {
        Self {
            conf_desc_len: 256,
            bos_desc_len: 256,
            ctrl_buf_len: 128
        }
    }
}

pub struct UsbConfig {
    usb_ven_id: u16,
    usb_prod_id: u16,
    usb_manufacturer: &'static str,
    usb_prod_name: &'static str,
    dev_rel: u16,
    serial_num: &'static str,
    composite_with_iads: bool,
    dev_class: u8,
}

// from hid cpp
impl UsbConfig {
    const fn new() -> Self {
        Self{
            usb_ven_id: 0x28DE,
            usb_prod_id:0x1142,
            usb_manufacturer: "Balve Software",
            usb_prod_name: "Steam Controller Puck",
            // todo - change the release ver back to wghat it was
            dev_rel: 0x1,
            // from identity - hardcoded this shit for now, replace with nrf silicon id later 
            serial_num: "FXB9960200000",
            // this needs to be set to false since we declaring device class
            // 0.000000 [ERROR] panicked at 'if composite_with_iads is set, you must set device_class = 0xEF, device_sub_class = 0x02, device_protocol = 0x01' (embassy_usb embassy-usb-0.6.0/src/builder.rs:179)
            composite_with_iads: false,
            dev_class: 0x00,
        }
    }
}
