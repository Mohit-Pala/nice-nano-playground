// state machine to diffbw fresh and repeat beacon polls

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SeqTracker { 
    last: Option<u8> 
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Freshness { 
    Fresh { missed: u8 }, 
    Repeat 
}

impl SeqTracker {
    pub const fn new() -> Self {
        Self { last: None }
    }

    pub fn observe(&mut self, seq: u8) -> Freshness {
        match self.last {
            // first sameple / after reset
            None => {
                self.last = Some(seq);
                Freshness::Fresh { missed: 0 }
            }
            // repeat
            Some(l) if l == seq => Freshness::Repeat,
            // new seq num
            Some(l) => {
                self.last = Some(seq);
                Freshness::Fresh {
                    missed: seq.wrapping_sub(l).wrapping_sub(1),
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.last = None;
    }

    pub fn last(&self) -> Option<u8> {
        self.last
    }
}

impl Freshness {
    pub const fn is_fresh(self) -> bool {
        match self {
            Self::Fresh { .. } => true,
            _ => false
        }
    }
}