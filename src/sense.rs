use esp_idf_svc::hal::i2c::I2cDriver;
use std::time::{Duration, Instant};

const EVENT_TIMEOUT: Duration = Duration::from_secs(1);

pub struct BoardChangeEvent {

}

pub struct Sense {
    i2c: I2cDriver<'static>,
    current_state: u64,
    previous_scan: u64,
    last_change: Option<Instant>,
    event_active: bool,
    event_mask: u64,
}

impl Sense {
    pub fn new(i2c: I2cDriver<'static>) -> Self {
        Self { 
            i2c, 
            previous_scan: 0u64, 
            current_state: 0u64,
            last_change: None,
            event_active: false,
            event_mask: 0,
        }
    }

    // one full board scan
    fn scan(&mut self) -> u64 {
        // 64-bitfield, 0 if piece is present.
        // will be 4 i2c 1x16 multiplexers that read back a value into some gpio pin
        0
    }

    // event processing
    fn tick(&mut self, now: Instant) -> Option<BoardChangeEvent> {
        let new_state = self.scan();
        let diff = self.current_state ^ new_state;
        if diff != 0 {
            self.current_state = new_state;

            if !self.event_active {
                self.event_active = true; 
                self.event_mask = 0; 
            }

            self.event_mask |= diff;
            self.last_change = Some(now);
        }

        if self.event_active {
            if let Some(last) = self.last_change {
                if now.duration_since(last) >= EVENT_TIMEOUT {
                    self.event_active = false;
                    self.event_mask = 0;
                    self.last_change = None;
                }
            }
        }  

        None
    }
}