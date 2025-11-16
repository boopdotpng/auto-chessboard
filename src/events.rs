use std::sync::mpsc::{channel, Receiver, Sender};

pub type EventSender = Sender<Event>;
pub type EventReceiver = Receiver<Event>;

pub trait EventHandler {
    fn handle(&mut self, evt: &Event) -> anyhow::Result<()>;
}

pub struct EventBus {
    sender: EventSender,
    receiver: EventReceiver,
    handlers: Vec<Box<dyn EventHandler + Send + 'static>>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, receiver) = channel(); 
        Self {
            sender,
            receiver, 
            handlers: Vec::new(),
        }
    }

    pub fn sender(&self) -> EventSender {
        self.sender.clone()
    }

    pub fn register_handler(&mut self, handler: Box<dyn EventHandler + Send + 'static>) {
        self.handlers.push(handler);
    }

    pub fn run(mut self) -> anyhow::Result<()> {
        while let Ok(event) = self.receiver.recv() {
            for handler in self.handlers.iter_mut() {
                if let Err(e) = handler.handle(&event) {
                    todo!();
                }
            }
        }

        Ok(())
    }
}

pub enum Event {
    // power / battery
    RequestBattery,
    BatteryReported {
        percent: u8,
        charging: bool,
    },

    // board state
    RequestBoardPosition,
    SetBoardPosition { fen: String },
    BoardPositionUpdated {fen: String },

    // moves
    MovePiece {
        from: u8,
        to: u8,
    },
    RequestPgn {
        pgn: String
    },

    // local hardware / motion 
    MotionCommand(CoreXyCommand),
    MotionFinished,
}

pub enum CoreXyCommand {
    Home,
    GotoMM {x: u32, y: u32},
}