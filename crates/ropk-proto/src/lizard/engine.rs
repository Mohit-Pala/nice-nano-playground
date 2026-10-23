use crate::bond::NUM_SLOTS;
use crate::hid::consumer::ConsumerReport;
use crate::hid::keyboard::KeyboardReport;
use crate::hid::mouse::MouseReport;
use crate::input::{ControllerInput, Sample};
use crate::lizard::buttons::LizardButtons;
use crate::lizard::eval::{eval, LizardOutputs};
use crate::lizard::map::LizardMap;
use crate::lizard::motion::{MotionConfig, PadMouse, PadScroll};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Desired {
    pub mouse: MouseReport,
    pub keyboard: KeyboardReport,
    pub consumer: ConsumerReport,
}

impl Desired {
    pub const NEUTRAL: Self = Self {
        mouse: MouseReport::NEUTRAL,
        keyboard: KeyboardReport::NEUTRAL,
        consumer: ConsumerReport::NEUTRAL,
    };

    pub fn is_neutral(&self) -> bool {
        *self == Self::NEUTRAL
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LizardEngine {
    pub map: LizardMap,
    pub config: MotionConfig,
    mouse: PadMouse,
    scroll: PadScroll,
}

impl LizardEngine {
    pub const fn new(map: LizardMap, config: MotionConfig) -> Self {
        Self {
            map,
            config,
            mouse: PadMouse::NEW,
            scroll: PadScroll::NEW,
        }
    }

    pub fn reset(&mut self) {
        self.mouse.reset();
        self.scroll.reset();
    }

    pub fn step(&mut self, slots: &[Option<Sample>; NUM_SLOTS], now_ms: u64) -> Desired {
        let live: [Option<&ControllerInput>; NUM_SLOTS] = core::array::from_fn(|i| {
            slots[i].as_ref().and_then(|sample| {
                if sample.is_fresh(now_ms) {
                    Some(&sample.input)
                } else {
                    None
                }
            })
        });

        let mut out = LizardOutputs::EMPTY;
        for &input_opt in &live {
            if let Some(input) = input_opt {
                eval(&self.map, LizardButtons::from_input(input), &mut out);
            }
        }

        let (x, y) = if out.mouse.is_some() {
            self.mouse.step(&live, &self.config)
        } else {
            (0, 0)
        };

        let wheel = if out.scroll.is_some() {
            self.scroll.step(&live, &self.config)
        } else {
            0
        };

        Desired {
            mouse: MouseReport {
                buttons: out.mouse_buttons,
                x,
                y,
                wheel,
                pan: 0,
            },
            keyboard: out.keyboard,
            consumer: ConsumerReport {
                controls: out.consumer,
            },
        }
    }
}
