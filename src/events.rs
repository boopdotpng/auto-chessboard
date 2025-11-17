/*
BLE protocol overview:
  * All frames are ASCII `CMD payload`.
  * Simple commands without data: `REQ_BAT`, `REQ_BOARD`, `REQ_PGN`.
  * Numeric data: `BAT <percent> <0|1>`, `MOVE <from> <to>`.
  * String data (FEN/PGN): `BOARD len:payload`, `SET_BOARD len:payload`, `PGN len:payload`
    where `len` is the byte-count of the UTF-8 payload that follows the colon.
BLE message variants map one-to-one with internal `Event` variants that need BLE I/O.
*/

use std::convert::TryFrom;
use std::fmt::Write as _;
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
    SendPgn { pgn: String },
}

#[derive(Debug)]
pub enum CodecError {
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
            BleMessage::SendPgn { pgn } => Event::SendPgn { pgn },
        }
    }
}

impl TryFrom<&Event> for BleMessage {
    type Error = ();

    fn try_from(evt: &Event) -> Result<Self, Self::Error> {
        match evt {
            Event::BatteryReported { percent, charging } =>
                Ok(BleMessage::BatteryReported { percent: *percent, charging: *charging }),
            Event::BoardPositionUpdated { fen } =>
                Ok(BleMessage::BoardPosition { fen: fen.clone() }),
            Event::SendPgn { pgn } =>
                Ok(BleMessage::SendPgn { pgn: pgn.clone() }),
            _ => Err(()),
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
                if let Err(_e) = handler.handle(&event) {
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

pub struct SimpleBleCodec;

impl SimpleBleCodec {
    fn encode_string_message(cmd: &str, payload: &str) -> Vec<u8> {
        let mut out = String::new();
        let _ = write!(&mut out, "{} {}:{}", cmd, payload.as_bytes().len(), payload);
        out.into_bytes()
    }

    fn decode_string_payload(payload: &str) -> Result<String, CodecError> {
        let (len_part, data_part) = payload
            .split_once(':')
            .ok_or(CodecError::Invalid)?;
        let expected_len: usize = len_part.parse().map_err(|_| CodecError::Invalid)?;
        if data_part.as_bytes().len() != expected_len {
            return Err(CodecError::Invalid);
        }
        Ok(data_part.to_string())
    }

    fn parse_bool(token: &str) -> Result<bool, CodecError> {
        match token {
            "0" => Ok(false),
            "1" => Ok(true),
            _ => Err(CodecError::Invalid),
        }
    }
}

impl BleCodec for SimpleBleCodec {
    type Wire = Vec<u8>;

    fn encode(msg: &BleMessage) -> Result<Self::Wire, CodecError> {
        Ok(match msg {
            BleMessage::RequestBattery => b"REQ_BAT".to_vec(),
            BleMessage::BatteryReported { percent, charging } => {
                let mut out = String::new();
                let _ = write!(
                    &mut out,
                    "BAT {} {}",
                    percent,
                    if *charging { 1 } else { 0 }
                );
                out.into_bytes()
            }
            BleMessage::RequestBoardPosition => b"REQ_BOARD".to_vec(),
            BleMessage::BoardPosition { fen } =>
                Self::encode_string_message("BOARD", fen),
            BleMessage::SetBoardPosition { fen } =>
                Self::encode_string_message("SET_BOARD", fen),
            BleMessage::MovePiece { from, to } => {
                let mut out = String::new();
                let _ = write!(&mut out, "MOVE {} {}", from, to);
                out.into_bytes()
            }
            BleMessage::RequestPgn => b"REQ_PGN".to_vec(),
            BleMessage::SendPgn { pgn } =>
                Self::encode_string_message("PGN", pgn),
        })
    }

    fn decode(bytes: &[u8]) -> Result<BleMessage, CodecError> {
        let text = core::str::from_utf8(bytes).map_err(|_| CodecError::Invalid)?;
        let (cmd, rest) = text
            .split_once(' ')
            .map(|(c, r)| (c, Some(r)))
            .unwrap_or((text, None));

        match cmd {
            "REQ_BAT" => Ok(BleMessage::RequestBattery),
            "BAT" => {
                let payload = rest.ok_or(CodecError::Invalid)?;
                let mut fields = payload.split_whitespace();
                let percent = fields
                    .next()
                    .ok_or(CodecError::Invalid)?
                    .parse::<u8>()
                    .map_err(|_| CodecError::Invalid)?;
                let charging = fields
                    .next()
                    .ok_or(CodecError::Invalid)
                    .and_then(Self::parse_bool)?;
                Ok(BleMessage::BatteryReported { percent, charging })
            }
            "REQ_BOARD" => Ok(BleMessage::RequestBoardPosition),
            "BOARD" => {
                let payload = rest.ok_or(CodecError::Invalid)?;
                let fen = Self::decode_string_payload(payload)?;
                Ok(BleMessage::BoardPosition { fen })
            }
            "SET_BOARD" => {
                let payload = rest.ok_or(CodecError::Invalid)?;
                let fen = Self::decode_string_payload(payload)?;
                Ok(BleMessage::SetBoardPosition { fen })
            }
            "MOVE" => {
                let payload = rest.ok_or(CodecError::Invalid)?;
                let mut fields = payload.split_whitespace();
                let from = fields
                    .next()
                    .ok_or(CodecError::Invalid)?
                    .parse::<u8>()
                    .map_err(|_| CodecError::Invalid)?;
                let to = fields
                    .next()
                    .ok_or(CodecError::Invalid)?
                    .parse::<u8>()
                    .map_err(|_| CodecError::Invalid)?;
                Ok(BleMessage::MovePiece { from, to })
            }
            "REQ_PGN" => Ok(BleMessage::RequestPgn),
            "PGN" => {
                let payload = rest.ok_or(CodecError::Invalid)?;
                let pgn = Self::decode_string_payload(payload)?;
                Ok(BleMessage::SendPgn { pgn })
            }
            _ => Err(CodecError::Invalid),
        }
    }
}
