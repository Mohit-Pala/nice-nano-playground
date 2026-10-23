use crate::hid::consumer::ConsumerControls;
use crate::hid::keyboard::{Key, Modifiers, MAX_KEYS};
use crate::hid::mouse::MouseButtons;
use crate::lizard::buttons::{LizardButtons, StickDir};
use crate::sc_controller_buttons::Button;

pub const MAX_BINDINGS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
                                                                    //name in c code
pub enum MouseSource {
    RightTrackpad,                                                  // LZ_MSRC_RPAD
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ScrollSource {
    LeftTrackpad,                                                   // LZ_MSRC_LPAD
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Output {
    KbdChord {
        modifiers: Modifiers,
        keys: [Key; MAX_KEYS],
    },                                                                 // LZ_OUT_KBD_CHORD
    MouseButton(MouseButtons),                                      // LZ_OUT_MOUSE_BTN
    MouseAxis(MouseSource),                                         // LZ_OUT_MOUSE_AXIS
    Scroll(ScrollSource),                                           // LZ_OUT_SCROLL
    Consumer(ConsumerControls),                                     // LZ_OUT_CONSUMER
}

impl Output {
    pub const fn chord(modifiers: Modifiers, key: Key) -> Output {
        Output::KbdChord {
            modifiers,
            keys: [key, Key::NONE, Key::NONE, Key::NONE, Key::NONE, Key::NONE],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Binding {
    pub output: Output,
    pub trigger: LizardButtons, // any of
    pub hold: LizardButtons,    // all oif
}

impl Binding {
    pub const fn new(output: Output, trigger: LizardButtons, hold: LizardButtons) -> Self {
        Self {
            output,
            trigger,
            hold,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct MapFull;

impl core::fmt::Display for MapFull {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("lizard map full")
    }
}

const PLACEHOLDER: Binding = Binding::new(
    Output::MouseButton(MouseButtons::EMPTY),
    LizardButtons::EMPTY,
    LizardButtons::EMPTY,
);

const STEAM: LizardButtons = LizardButtons::button(Button::Steam);
const NONE: LizardButtons = LizardButtons::EMPTY;

const fn btn(b: Button) -> LizardButtons {
    LizardButtons::button(b)
}

const fn stick(d: StickDir) -> LizardButtons {
    LizardButtons::stick(d)
}

const fn plain_key(k: Key, trig: LizardButtons) -> Binding {
    Binding::new(Output::chord(Modifiers::EMPTY, k), trig, NONE)
}

pub const DEFAULT_MAP: [Binding; 26] = [
    // analog
    Binding::new(Output::MouseAxis(MouseSource::RightTrackpad), NONE, NONE),
    Binding::new(Output::Scroll(ScrollSource::LeftTrackpad), NONE, NONE),


    // mouse btns
    Binding::new(
        Output::MouseButton(MouseButtons::LEFT),
        btn(Button::RTrackpadClick),
        NONE,
    ),
    Binding::new(Output::MouseButton(MouseButtons::LEFT), btn(Button::RT), NONE),
    Binding::new(Output::MouseButton(MouseButtons::RIGHT), btn(Button::LT), NONE),
    Binding::new(
        Output::MouseButton(MouseButtons::MIDDLE),
        btn(Button::LTrackpadClick),
        NONE,
    ),



    // \\\
    Binding::new(
        Output::chord(Modifiers::LCTRL.union(Modifiers::LALT), Key::DELETE),
        btn(Button::L4),
        STEAM,
    ),
    Binding::new(
        Output::chord(Modifiers::LGUI.union(Modifiers::LCTRL), Key::O),
        btn(Button::X),
        STEAM,
    ),
    Binding::new(Output::Consumer(ConsumerControls::VOL_DOWN), btn(Button::L5), STEAM),
    Binding::new(Output::Consumer(ConsumerControls::VOL_UP), btn(Button::R5), STEAM),



    // btns
    plain_key(Key::ENTER, btn(Button::A)),
    plain_key(Key::ESCAPE, btn(Button::B)),
    plain_key(Key::PAGE_UP, btn(Button::X)),
    plain_key(Key::PAGE_DOWN, btn(Button::Y)),
    plain_key(Key::TAB, btn(Button::Select)),
    plain_key(Key::ESCAPE, btn(Button::Start)),


    // dpad
    plain_key(Key::ARROW_UP, btn(Button::DpadUp)),
    plain_key(Key::ARROW_DOWN, btn(Button::DpadDown)),
    plain_key(Key::ARROW_LEFT, btn(Button::DpadLeft)),
    plain_key(Key::ARROW_RIGHT, btn(Button::DpadRight)),




    // lstick
    plain_key(Key::ARROW_UP, stick(StickDir::Up)),
    plain_key(Key::ARROW_DOWN, stick(StickDir::Down)),
    plain_key(Key::ARROW_LEFT, stick(StickDir::Left)),
    plain_key(Key::ARROW_RIGHT, stick(StickDir::Right)),




    // 
    Binding::new(Output::chord(Modifiers::LCTRL, Key::NONE), btn(Button::Lb), NONE),
    Binding::new(Output::chord(Modifiers::LALT, Key::NONE), btn(Button::Rb), NONE),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct LizardMap {
    bindings: [Binding; MAX_BINDINGS], 
    len: u8,
}

impl LizardMap {
    pub const EMPTY: Self = Self {
        bindings: [PLACEHOLDER; MAX_BINDINGS],
        len: 0,
    };

    pub const fn default_map() -> Self {
        let mut m = Self::EMPTY;
        let mut i = 0;
        while i < DEFAULT_MAP.len() {
            m.bindings[i] = DEFAULT_MAP[i];
            i += 1;
        }
        m.len = DEFAULT_MAP.len() as u8;
        m
    }

    pub fn from_slice(bindings: &[Binding]) -> Result<Self, MapFull> {
        if bindings.len() > MAX_BINDINGS {
            return Err(MapFull);
        }
        let mut m = Self::EMPTY;
        let mut i = 0;
        while i < bindings.len() {
            m.bindings[i] = bindings[i];
            i += 1;
        }
        m.len = bindings.len() as u8;
        Ok(m)
    }

    pub fn push(&mut self, b: Binding) -> Result<(), MapFull> {
        if (self.len as usize) >= MAX_BINDINGS {
            return Err(MapFull);
        }
        self.bindings[self.len as usize] = b;
        self.len += 1;
        Ok(())
    }

    pub fn bindings(&self) -> &[Binding] {
        &self.bindings[..self.len as usize]
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn is_full(&self) -> bool {
        self.len as usize >= MAX_BINDINGS
    }
}
