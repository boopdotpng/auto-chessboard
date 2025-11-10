// all the events in our application
use crate::sense::BoardChangeEvent;
use std::sync::mpsc::{channel, Receiver, Sender};

pub enum Event {
    Bt(BtEvent),
    Move(CoreXyEvent),
    Engine(EngineEvent),
    BoardChanged(BoardChangeEvent),
}

pub type EventSender = Sender<Event>;
pub type EventReceiver = Receiver<Event>;

pub fn bus() -> (EventSender, EventReceiver) {
    channel()
}

pub fn dispatcher(rx: EventReceiver, bt_tx: EventSender) -> anyhow::Result<()> {
    for event in rx {
        match event {
            Event::Bt(bt_event) => {
                // do things
                bt_tx.send(Event::Bt(bt_event))?;
            }
            Event::Move(_core_event) => {
                // do things
            }
            Event::Engine(_engine_event) => {
                // do things
            }
            Event::BoardChanged(_board_event) => {
                // do things
            }
        }
    }

    Ok(())
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
