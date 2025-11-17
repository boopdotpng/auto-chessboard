use std::fmt;

use crate::util::square_to_coord;

pub type Bitboard = u64;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub(crate) fn idx(self) -> usize {
        match self {
            Color::White => 0,
            Color::Black => 1,
        }
    }

    pub(crate) fn opponent(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }

    pub(crate) fn from_char(ch: char) -> Option<Color> {
        match ch {
            'P' | 'N' | 'B' | 'R' | 'Q' | 'K' => Some(Color::White),
            'p' | 'n' | 'b' | 'r' | 'q' | 'k' => Some(Color::Black),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PieceKind {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl PieceKind {
    pub(crate) const ALL: [PieceKind; 6] = [
        PieceKind::Pawn,
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Rook,
        PieceKind::Queen,
        PieceKind::King,
    ];

    pub(crate) fn fen_symbol(self, color: Color) -> char {
        let sym = match self {
            PieceKind::Pawn => 'p',
            PieceKind::Knight => 'n',
            PieceKind::Bishop => 'b',
            PieceKind::Rook => 'r',
            PieceKind::Queen => 'q',
            PieceKind::King => 'k',
        };
        match color {
            Color::White => sym.to_ascii_uppercase(),
            Color::Black => sym,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CastleSide {
    KingSide,
    QueenSide,
}

impl CastleSide {
    pub(crate) fn as_bits(self, color: Color) -> u8 {
        match (color, self) {
            (Color::White, CastleSide::KingSide) => 0b0001,
            (Color::White, CastleSide::QueenSide) => 0b0010,
            (Color::Black, CastleSide::KingSide) => 0b0100,
            (Color::Black, CastleSide::QueenSide) => 0b1000,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Move {
    pub color: Color,
    pub piece: PieceKind,
    pub from: u8,
    pub to: u8,
    pub capture: Option<PieceKind>,
    pub capture_square: Option<u8>,
    pub castle: Option<CastleSide>,
    pub is_en_passant: bool,
    pub is_double_pawn_push: bool,
    pub promotion: Option<PieceKind>,
    pub requires_promotion: bool,
}

impl Move {
    pub(crate) fn coord_string(&self) -> String {
        if let Some(side) = self.castle {
            return match side {
                CastleSide::KingSide => "O-O".to_string(),
                CastleSide::QueenSide => "O-O-O".to_string(),
            };
        }
        let mut out = String::new();
        out.push_str(&square_to_coord(self.from));
        out.push('-');
        out.push_str(&square_to_coord(self.to));
        if let Some(promo) = self.promotion {
            out.push('=');
            out.push(promo.fen_symbol(self.color));
        }
        out
    }
}

#[derive(Debug)]
pub enum EngineError {
    InvalidFen(String),
    InvalidMask(String),
    IllegalMove(String),
    PendingPromotion,
    Square(String),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::InvalidFen(msg) => write!(f, "invalid FEN: {msg}"),
            EngineError::InvalidMask(msg) => write!(f, "invalid mask: {msg}"),
            EngineError::IllegalMove(msg) => write!(f, "illegal move: {msg}"),
            EngineError::PendingPromotion => write!(f, "promotion pending"),
            EngineError::Square(msg) => write!(f, "invalid square: {msg}"),
        }
    }
}

impl std::error::Error for EngineError {}

#[derive(Clone, Debug)]
pub struct MoveSummary {
    pub mv: Move,
    pub fen: String,
    pub pgn: String,
}

#[derive(Clone, Debug)]
pub struct PromotionRequest {
    pub color: Color,
    pub square: u8,
}

#[derive(Clone, Debug)]
pub enum EngineUpdate {
    NoChange,
    MoveApplied(MoveSummary),
    PromotionPending(PromotionRequest),
}
