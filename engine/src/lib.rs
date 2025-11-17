mod board;
mod change;
mod types;
mod util;

pub use crate::types::{
    Bitboard, CastleSide, Color, EngineError, EngineUpdate, Move, MoveSummary, PieceKind,
    PromotionRequest,
};

use crate::board::Board;
use crate::change::ChangeSet;
use crate::util::coord_to_square;

struct PendingPromotion {
    plan: Move,
}

pub struct Engine {
    board: Board,
    history: Vec<Move>,
    pending: Option<PendingPromotion>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            board: Board::starting_position(),
            history: Vec::new(),
            pending: None,
        }
    }

    pub fn from_fen(fen: &str) -> Result<Self, EngineError> {
        Ok(Self {
            board: Board::from_fen(fen)?,
            history: Vec::new(),
            pending: None,
        })
    }

    pub fn set_position(&mut self, fen: &str) -> Result<(), EngineError> {
        self.board = Board::from_fen(fen)?;
        self.history.clear();
        self.pending = None;
        Ok(())
    }

    pub fn to_fen(&self) -> String {
        self.board.to_fen()
    }

    pub fn pgn(&self) -> String {
        build_pgn(&self.history)
    }

    pub fn occupancy_mask(&self) -> Bitboard {
        self.board.occupancy()
    }

    pub fn piece_at(&self, square: u8) -> Option<(Color, PieceKind)> {
        self.board.piece_at(square)
    }

    pub fn square_from_coord(coord: &str) -> Result<u8, EngineError> {
        coord_to_square(coord).ok_or_else(|| EngineError::Square(coord.to_string()))
    }

    pub fn observe(&mut self, mask: Bitboard, state: Bitboard) -> Result<EngineUpdate, EngineError> {
        if self.pending.is_some() {
            return Err(EngineError::PendingPromotion);
        }

        let expected = self.board.occupancy();
        if expected == state {
            return Ok(EngineUpdate::NoChange);
        }

        let change = ChangeSet::new(mask, expected, state, &self.board)?;
        if !change.represents_move() {
            return Ok(EngineUpdate::NoChange);
        }

        let intent = change.to_intent(&self.board)?;
        let plan = self.board.validate_intent(intent)?;
        let mut board_clone = self.board.clone();
        board_clone.apply_move(&plan);
        if board_clone.occupancy() != state {
            return Err(EngineError::InvalidMask("state does not match move".into()));
        }

        self.board = board_clone;
        if plan.requires_promotion {
            self.pending = Some(PendingPromotion { plan });
            return Ok(EngineUpdate::PromotionPending(PromotionRequest {
                color: plan.color,
                square: plan.to,
            }));
        }

        let summary = self.finalize_move(plan);
        Ok(EngineUpdate::MoveApplied(summary))
    }

    pub fn confirm_promotion(&mut self, piece: PieceKind) -> Result<MoveSummary, EngineError> {
        let mut pending = self.pending.take().ok_or(EngineError::IllegalMove(
            "no pending promotion".into(),
        ))?;
        if pending.plan.piece != PieceKind::Pawn {
            return Err(EngineError::IllegalMove(
                "pending move is not a pawn promotion".into(),
            ));
        }
        if matches!(piece, PieceKind::Pawn | PieceKind::King) {
            return Err(EngineError::IllegalMove(
                "promotion must be to knight, bishop, rook, or queen".into(),
            ));
        }

        pending.plan.promotion = Some(piece);
        pending.plan.requires_promotion = false;
        self.board.promote_piece(pending.plan.to, piece)?;
        let summary = self.finalize_move(pending.plan);
        Ok(summary)
    }

    fn finalize_move(&mut self, mv: Move) -> MoveSummary {
        self.history.push(mv);
        MoveSummary {
            mv,
            fen: self.board.to_fen(),
            pgn: self.pgn(),
        }
    }
}

fn build_pgn(history: &[Move]) -> String {
    let mut out = String::new();
    for (idx, mv) in history.iter().enumerate() {
        if idx % 2 == 0 {
            if !out.is_empty() {
                out.push(' ');
            }
            let turn = idx / 2 + 1;
            out.push_str(&format!("{turn}.{}", mv.coord_string()));
        } else {
            out.push(' ');
            out.push_str(&mv.coord_string());
        }
    }
    out
}
