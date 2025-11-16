// all the events in our application
use crate::sense::BoardChangeEvent;
use std::sync::mpsc::{channel, Receiver, Sender};

pub enum CodecError {
    TooLarge,
    Invalid
}

// turn any event into a string or bytes to send over bluetooth
pub trait EventCodec: Sized {
    type Wire: AsRef<[u8]> + From<Vec<u8>>;
    fn encode(&self) -> Result<Self::Wire, CodecError>;
    fn decode(bytes: &[u8]) -> Result<Self, CodecError>;
}

pub trait EventHandler {
    fn interested_in(&self) -> &'static [EventKind];
    fn handle(&mut self, evt: &Event) -> anyhow::Result<()>;
}

pub enum Event {
    Bt(BtEvent),
    Move(CoreXyEvent),
    Engine(EngineEvent),
    BoardChanged(BoardChangeEvent),
}

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub enum EventKind {
    Bt, 
    CoreXY, 
    Engine, 
    BoardChanged,
}

impl From<&Event> for EventKind {
    fn from(value: &Event) -> Self {
        match value {
            Event::Bt(_) => EventKind::Bt,
            Event::Move(_) => EventKind::CoreXY,
            Event::Engine(_) => EventKind::Engine,
            Event::BoardChanged(_) => EventKind::BoardChanged,
        }
    }
}

pub type EventSender = Sender<Event>;
pub type EventReceiver = Receiver<Event>;

pub struct EventBus {
    sender: EventSender,
    receiver: EventReceiver,
    handlers: Vec<Box<dyn EventHandler + Send>>,
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

    pub fn register_handler(&mut self, handler: Box<dyn EventHandler + Send>) {
        self.handlers.push(handler);
    }

    pub fn run(mut self) -> anyhow::Result<()> {
        while let Ok(event) = self.receiver.recv() {
            let kind = EventKind::from(&event);
            for handler in self.handlers.iter_mut() {
                if handler.interested_in().iter().any(|k| *k == kind) {
                    let _ = handler.handle(&event);
                }
            }
        }
        Ok(())
    }
}

// events sent from the phone app to the esp32
pub enum BtEvent {
    // core functionality
    Pause, // stop doing things -- become a dumb chess board
    Sleep, // battery savings -- "off"
    ReportBattery, // report battery % and charging status

    CoreXyEvent(CoreXyEvent),
    EngineEvent(EngineEvent),
}

pub enum CoreXyEvent {
    Home, // go to 0,0 and stop
    ManualPieceMove, // move a piece from square to square manually  
    Goto // go to certain position (x,y)
}

pub enum EngineEvent {
    MovePlayed(String), // the user plays a move 
    EngineMoveSent(String), // the engine (on the phone) sends the next best move back
    SetBoard(String),  // set engine to a FEN position (initial or custom position)
    GetFEN(String), // get the FEN position from the engine
    GetPGN(String), // get the PGN from the engine
    ReplayPGN(String), // send PGN for the board to replay 
}
