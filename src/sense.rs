use esp_idf_svc::hal::delay::BLOCK;
use esp_idf_svc::hal::i2c::I2cDriver;
use std::time::{Duration, Instant};

const EVENT_TIMEOUT: Duration = Duration::from_secs(1);
const PCF_BASE_ADDR: u8 = 0x20;

pub struct BoardChange {
    pub mask: u64,
    pub state: u64,
}

pub struct Sense {
    i2c: I2cDriver<'static>,
    current_state: u64,
    last_change: Option<Instant>,
    event_active: bool,
    event_mask: u64,
}

impl Sense {
    pub fn new(i2c: I2cDriver<'static>) -> Self {
        Self {
            i2c,
            current_state: 0u64,
            last_change: None,
            event_active: false,
            event_mask: 0,
        }
    }

    // you need to set each pin on the exapnder as an inputs 
    fn init_as_inputs(&mut self) {
        let all_high = [0xFFu8, 0xFFu8];
        for i in 0..4 {
            let addr = PCF_BASE_ADDR + i as u8;
            let _ = self.i2c.write(addr, &all_high, BLOCK);
        }
    }

    // one full board scan
    fn scan(&mut self) -> u64 {
        // 64-bitfield, 1 if piece is present.
        // will be 4 i2c 1x16 multiplexers that read back a value into some gpio pin (PCF8575)
        // address 0x20 to 0x27 dependinng on how a0-a2 are configured.
        // todo! verify actual addresses
        let mut state = 0u64;
        for i in 0..4 {
            let addr = PCF_BASE_ADDR + i as u8;
            // 2 bytes: 16 values
            let mut buf = [0u8; 2];
            self.i2c.read(addr, &mut buf, BLOCK);

            state |= (u16::from_le_bytes(buf) as u64) << ( (i*16) as u32 );
        }

        state
    }

    // event processing
    fn tick(&mut self, now: Instant) -> Option<BoardChange> {
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
                    let mask = self.event_mask;
                    self.event_mask = 0;
                    self.last_change = None;
                    return Some(BoardChange {
                        mask,
                        state: self.current_state,
                    });
                }
            }
        }

        None
    }
}