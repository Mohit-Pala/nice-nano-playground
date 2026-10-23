use crate::hid::consumer::{ConsumerReport, CONSUMER_REPORT_LEN};
use crate::hid::keyboard::{KeyboardReport, KEYBOARD_REPORT_LEN};
use crate::hid::mouse::{MouseButtons, MouseReport, MOUSE_REPORT_LEN};
use crate::lizard::engine::Desired;

pub const MAX_REPORT_LEN: usize = KEYBOARD_REPORT_LEN;
pub const MAX_PENDING_MOTION: i32 = 127;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ReportKind {
    Mouse,
    Keyboard,
    Consumer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum OutReport {
    Mouse(MouseReport),
    Keyboard(KeyboardReport),
    Consumer(ConsumerReport),
}

impl OutReport {
    pub fn kind(&self) -> ReportKind {
        match self {
            OutReport::Mouse(_) => ReportKind::Mouse,
            OutReport::Keyboard(_) => ReportKind::Keyboard,
            OutReport::Consumer(_) => ReportKind::Consumer,
        }
    }

    pub fn encode(&self, buf: &mut [u8]) -> usize {
        match self {
            OutReport::Mouse(m) => {
                let bytes = m.encode();
                buf[..MOUSE_REPORT_LEN].copy_from_slice(&bytes);
                MOUSE_REPORT_LEN
            }
            OutReport::Keyboard(k) => {
                let bytes = k.encode();
                buf[..KEYBOARD_REPORT_LEN].copy_from_slice(&bytes);
                KEYBOARD_REPORT_LEN
            }
            OutReport::Consumer(c) => {
                let bytes = c.encode();
                buf[..CONSUMER_REPORT_LEN].copy_from_slice(&bytes);
                CONSUMER_REPORT_LEN
            }
        }
    }
}

const KINDS: [ReportKind; 3] = [
    ReportKind::Mouse,
    ReportKind::Keyboard,
    ReportKind::Consumer,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Delivery {
    with_consumer: bool,
    want_buttons: MouseButtons,
    want_kbd: KeyboardReport,
    want_cc: ConsumerReport,
    motion: [i32; 4],
    got_buttons: Option<MouseButtons>,
    got_kbd: Option<KeyboardReport>,
    got_cc: Option<ConsumerReport>,
    flight: Option<OutReport>,
    last: ReportKind,
}

impl Delivery {
    pub const fn new(with_consumer: bool) -> Self {
        Self {
            with_consumer,
            want_buttons: MouseButtons::EMPTY,
            want_kbd: KeyboardReport::NEUTRAL,
            want_cc: ConsumerReport::NEUTRAL,
            motion: [0; 4],
            got_buttons: Some(MouseButtons::EMPTY),
            got_kbd: Some(KeyboardReport::NEUTRAL),
            got_cc: Some(ConsumerReport::NEUTRAL),
            flight: None,
            last: ReportKind::Consumer,
        }
    }

    pub fn set_desired(&mut self, d: &Desired) {
        self.want_buttons = d.mouse.buttons;
        self.want_kbd = d.keyboard;
        if self.with_consumer {
            self.want_cc = d.consumer;
        }

        self.motion[0] = (self.motion[0] + d.mouse.x as i32)
            .clamp(-MAX_PENDING_MOTION, MAX_PENDING_MOTION);
        self.motion[1] = (self.motion[1] + d.mouse.y as i32)
            .clamp(-MAX_PENDING_MOTION, MAX_PENDING_MOTION);
        self.motion[2] = (self.motion[2] + d.mouse.wheel as i32)
            .clamp(-MAX_PENDING_MOTION, MAX_PENDING_MOTION);
        self.motion[3] = (self.motion[3] + d.mouse.pan as i32)
            .clamp(-MAX_PENDING_MOTION, MAX_PENDING_MOTION);
    }

    pub fn is_pending(&self, kind: ReportKind) -> bool {
        match kind {
            ReportKind::Mouse => {
                self.got_buttons != Some(self.want_buttons)
                    || self.motion[0] != 0
                    || self.motion[1] != 0
                    || self.motion[2] != 0
                    || self.motion[3] != 0
            }
            ReportKind::Keyboard => self.got_kbd != Some(self.want_kbd),
            ReportKind::Consumer => self.with_consumer && self.got_cc != Some(self.want_cc),
        }
    }

    pub fn in_flight(&self) -> Option<ReportKind> {
        self.flight.as_ref().map(|r| r.kind())
    }

    pub fn is_settled(&self) -> bool {
        self.flight.is_none()
            && !self.is_pending(ReportKind::Mouse)
            && !self.is_pending(ReportKind::Keyboard)
            && !self.is_pending(ReportKind::Consumer)
    }

    pub fn next(&mut self) -> Option<OutReport> {
        if self.flight.is_some() {
            return None;
        }

        let last_idx = match self.last {
            ReportKind::Mouse => 0,
            ReportKind::Keyboard => 1,
            ReportKind::Consumer => 2,
        };

        for step in 1..=3 {
            let kind = KINDS[(last_idx + step) % 3];
            if self.is_pending(kind) {
                self.last = kind;
                let snapshot = match kind {
                    ReportKind::Mouse => OutReport::Mouse(MouseReport {
                        buttons: self.want_buttons,
                        x: self.motion[0] as i8,
                        y: self.motion[1] as i8,
                        wheel: self.motion[2] as i8,
                        pan: self.motion[3] as i8,
                    }),
                    ReportKind::Keyboard => OutReport::Keyboard(self.want_kbd),
                    ReportKind::Consumer => OutReport::Consumer(self.want_cc),
                };
                self.flight = Some(snapshot);
                return Some(snapshot);
            }
        }

        None
    }

    pub fn on_sent(&mut self, ok: bool) {
        let Some(flight) = self.flight.take() else {
            return;
        };

        if ok {
            match flight {
                OutReport::Mouse(m) => {
                    self.got_buttons = Some(m.buttons);
                    self.motion[0] -= m.x as i32;
                    self.motion[1] -= m.y as i32;
                    self.motion[2] -= m.wheel as i32;
                    self.motion[3] -= m.pan as i32;
                }
                OutReport::Keyboard(k) => {
                    self.got_kbd = Some(k);
                }
                OutReport::Consumer(c) => {
                    self.got_cc = Some(c);
                }
            }
        } else {
            match flight {
                OutReport::Mouse(_) => {
                    self.got_buttons = None;
                }
                OutReport::Keyboard(_) => {
                    self.got_kbd = None;
                }
                OutReport::Consumer(_) => {
                    self.got_cc = None;
                }
            }
        }
    }

    pub fn release_all(&mut self) {
        self.want_buttons = MouseButtons::EMPTY;
        self.want_kbd = KeyboardReport::NEUTRAL;
        self.want_cc = ConsumerReport::NEUTRAL;
        self.motion = [0; 4];
    }

    pub fn on_bus_reset(&mut self) {
        self.flight = None;
        self.motion = [0; 4];
        self.got_buttons = Some(MouseButtons::EMPTY);
        self.got_kbd = Some(KeyboardReport::NEUTRAL);
        self.got_cc = Some(ConsumerReport::NEUTRAL);
    }
}
