use std::sync::mpsc::{channel, Receiver, Sender};

pub type EventSender = Sender<Event>;
pub type EventReceiver = Receiver<Event>;

pub enum BleMessage {
    RequestBattery,
    BatteryReported { percent: u8, charging: bool },

    RequestBoardPosition,
    BoardPosition { fen: String },     // response
    SetBoardPosition { fen: String },

    MovePiece { from: u8, to: u8 },

    RequestPgn,
}

pub enum CodecError {
    TooLarge,
    Invalid,
}

pub trait BleCodec {
    type Wire: AsRef<[u8]> + From<Vec<u8>>;
    fn encode(msg: &BleMessage) -> Result<Self::Wire, CodecError>;
    fn decode(bytes: &[u8]) -> Result<BleMessage, CodecError>;
}

impl From<BleMessage> for Event {
    fn from(msg: BleMessage) -> Self {
        match msg {
            BleMessage::RequestBattery => Event::RequestBattery,
            BleMessage::BatteryReported { percent, charging } =>
                Event::BatteryReported { percent, charging },

            BleMessage::RequestBoardPosition => Event::RequestBoardPosition,
            BleMessage::BoardPosition { fen } =>
                Event::BoardPositionUpdated { fen },

            BleMessage::SetBoardPosition { fen } =>
                Event::SetBoardPosition { fen },

            BleMessage::MovePiece { from, to } =>
                Event::MovePiece { from, to },
            
            BleMessage::RequestPgn => Event::RequestPgn,
        }
    }
}

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

/*
typical flow for a bluetooth event:
phone sends RequestPgn.
bluetooth handler receives it, publishes RequestPgn on the bus,
GameState consumes it, builds the PGN, then emits SendPgn { pgn }
which the bluetooth handler forwards back to the phone.
*/
pub enum Event {
    // --- telemetry / BLE round-trips ---
    RequestBattery,
    BatteryReported {
        percent: u8,
        charging: bool,
    },
    RequestBoardPosition,
    BoardPositionUpdated {
        fen: String,
    },
    RequestPgn,
    SendPgn {
        pgn: String,
    },

    // --- board / engine coordination ---
    SetBoardPosition {
        fen: String,
    },
    MovePiece {
        from: u8,
        to: u8,
    },

    // --- local hardware / motion ---
    MotionCommand(CoreXyCommand),
    MotionFinished,
}

pub enum CoreXyCommand {
    Home,
    GotoMM {x: u32, y: u32},
}
