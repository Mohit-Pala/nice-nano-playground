use static_cell::StaticCell;
use embassy_usb::{Builder, UsbDevice, class::hid::{Config as HidConfig, HidBootProtocol, HidReaderWriter, HidSubclass, State as HidState}, driver::Driver};
use crate::{sc_default_descriptor::PUCK_HID_DESC, sc_puck_bond::ScPuckSlot, vars::{DUMMY_USB_CLASS, DUMMY_USB_PROTOCOL, DUMMY_USB_SUBCLASS, MAX_PCKT_SIZE}};

pub struct UsbDescLens {
    conf_desc_len: u16,
    bos_desc_len: u16,
    ctrl_buf_len: u16
}

impl UsbDescLens {
    pub const fn new() -> Self {
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
    pub const fn new() -> Self {
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

    // todo: this needs to be wired in
    pub const fn with_serial_number(mut self, serial_num: &'static str) -> Self {
        self.serial_num = serial_num;
        return self;
    }

    pub fn to_embassy_usb_conf(&self) -> embassy_usb::Config<'static> {
        let mut usb_config = embassy_usb::Config::new(self.usb_ven_id, self.usb_prod_id);
        usb_config.manufacturer = Some(self.usb_manufacturer);
        usb_config.product = Some(self.usb_prod_name);
        usb_config.composite_with_iads = self.composite_with_iads;
        usb_config.device_class = self.dev_class;
        usb_config.device_release = self.dev_rel;
        usb_config.serial_number = Some(self.serial_num);
        return usb_config;
    }
}

// rename ts
pub struct PuckSlotStruct {
    handler: StaticCell<ScPuckSlot>,
    state: StaticCell<HidState<'static>>,
}

impl PuckSlotStruct {
    pub const fn new() -> Self {
        Self {
            handler: StaticCell::new(),
            state: StaticCell::new(),
        }
    }
}

// this will be the 
// rename ts
// shit needed for https://docs.embassy.dev/embassy-usb/git/default/struct.Builder.html
pub struct PuckUsbStruct {
    config_desc: StaticCell<[u8; 256]>,
    bos_desc: StaticCell<[u8; 256]>,
    control_buf: StaticCell<[u8; 128]>,
    slots: [PuckSlotStruct; 4],
}

impl PuckUsbStruct {
    pub const fn new() -> Self {
        Self {
            config_desc: StaticCell::new(),
            bos_desc: StaticCell::new(),
            control_buf: StaticCell::new(),
            slots: [
                PuckSlotStruct::new(),
                PuckSlotStruct::new(),
                PuckSlotStruct::new(),
                PuckSlotStruct::new(),
            ],
        }
    }
}

// https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html#example
// https://rust-unofficial.github.io/patterns/patterns/creational/builder.html

pub struct PuckUsbBuilder<DihRiver: Driver<'static>> {
    // dih-river 😭😭😭 
    // todo change ts before commmitting lmao 
    puck_builder: Builder<'static, DihRiver>,
}

impl <DihRiver: Driver<'static>> PuckUsbBuilder<DihRiver> {
    pub fn new(driver: DihRiver, config: &UsbConfig, storage: &'static PuckUsbStruct) -> Self {

        let puck_builder = Builder::new(
            driver,
            config.to_embassy_usb_conf(),
            storage.config_desc.init([0; 256]),
            storage.bos_desc.init([0; 256]),
            &mut [],
            storage.control_buf.init([0; 128]),
        );
        return Self { puck_builder };
    }

    // 2 padding dummies
    // needeed for slots 2-5 to be pop
    pub fn add_dummy_slots(&mut self) -> &mut Self {
        for _ in 0..2 {
            let mut func = self.puck_builder.function(DUMMY_USB_CLASS, DUMMY_USB_SUBCLASS, DUMMY_USB_PROTOCOL);
            let _iface = func.interface();
        }
        return self;
    }

    pub fn add_slot(
        &mut self,
        slot: &'static PuckSlotStruct,
    ) -> HidReaderWriter<'static, DihRiver, 64, 64> {
        let handler = slot.handler.init(ScPuckSlot::new());
        let state = slot.state.init(HidState::new());
        let hid_config = HidConfig {
            report_descriptor: PUCK_HID_DESC,
            request_handler: Some(handler),
            poll_ms: 1,
            max_packet_size: 64,
            hid_boot_protocol: HidBootProtocol::None,
            hid_subclass: HidSubclass::No,
        };
        HidReaderWriter::<DihRiver, 64, 64>::new(&mut self.puck_builder, state, hid_config)
    }

    pub fn add_slots_0_3(
        &mut self,
        storage: &'static PuckUsbStruct,
    ) -> [HidReaderWriter<'static, DihRiver, 64, 64>; 4] {
        [
            self.add_slot(&storage.slots[0]),
            self.add_slot(&storage.slots[1]),
            self.add_slot(&storage.slots[2]),
            self.add_slot(&storage.slots[3]),
        ]
    }

    pub fn build(self) -> UsbDevice<'static, DihRiver> {
        self.puck_builder.build()
    }
}

pub fn build_sc_puck_usb<DihRiver: Driver<'static>>(
    driver: DihRiver,
    storage: &'static PuckUsbStruct,
) -> UsbDevice<'static, DihRiver> {
    let mut builder = PuckUsbBuilder::new(driver, &UsbConfig::new(), storage);
    builder.add_dummy_slots();
    let _slots = builder.add_slots_0_3(storage);
    builder.build()
}