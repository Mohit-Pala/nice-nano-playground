use crate::hid::consumer::ConsumerControls;
use crate::hid::keyboard::KeyboardReport;
use crate::hid::mouse::MouseButtons;
use crate::lizard::buttons::LizardButtons;
use crate::lizard::map::{LizardMap, MouseSource, Output, ScrollSource};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct LizardOutputs {
    pub keyboard: KeyboardReport,
    pub mouse_buttons: MouseButtons,
    pub consumer: ConsumerControls,
    pub mouse: Option<MouseSource>,
    pub scroll: Option<ScrollSource>,
}

impl LizardOutputs {
    pub const EMPTY: Self = Self {
        keyboard: KeyboardReport::NEUTRAL,
        mouse_buttons: MouseButtons::EMPTY,
        consumer: ConsumerControls::EMPTY,
        mouse: None,
        scroll: None,
    };
}

pub fn eval(map: &LizardMap, buttons: LizardButtons, out: &mut LizardOutputs) {
    let mut claimed = LizardButtons::EMPTY;

    for binding in map.bindings() {
        match &binding.output {
            Output::MouseAxis(src) => {
                out.mouse = Some(*src);
                continue;
            }
            Output::Scroll(src) => {
                out.scroll = Some(*src);
                continue;
            }
            _ => {}
        }

        if binding.trigger.is_empty() {
            continue;
        }
        if !buttons.contains(binding.hold) {
            continue;
        }
        if !buttons.intersects(binding.trigger) {
            continue;
        }

        match &binding.output {
            Output::MouseButton(m) => {
                out.mouse_buttons = out.mouse_buttons | *m;
            }
            Output::Consumer(c) => {
                out.consumer = out.consumer | *c;
            }
            Output::KbdChord { modifiers, keys } => {
                if binding.hold.is_empty() && binding.trigger.intersects(claimed) {
                    continue;
                }
                out.keyboard.modifiers = out.keyboard.modifiers | *modifiers;
                for &k in keys {
                    if k.is_none() {
                        break;
                    }
                    out.keyboard.press(k);
                }
                if !binding.hold.is_empty() {
                    claimed = claimed | binding.trigger;
                }
            }
            Output::MouseAxis(_) | Output::Scroll(_) => unreachable!(),
        }
    }
}
