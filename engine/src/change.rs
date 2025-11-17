use crate::board::Board;
use crate::types::{Bitboard, CastleSide, EngineError};
use crate::util::bit;

#[derive(Clone, Copy)]
pub(crate) enum MoveIntent {
    Standard {
        from: u8,
        to: u8,
        capture_square: Option<u8>,
    },
    Castle(CastleSide),
}

pub(crate) struct ChangeSet {
    removed_self: Vec<u8>,
    removed_enemy: Vec<u8>,
    added: Vec<u8>,
    replaced: Vec<u8>,
}

impl ChangeSet {
    pub(crate) fn new(
        mask: Bitboard,
        previous: Bitboard,
        state: Bitboard,
        board: &Board,
    ) -> Result<Self, EngineError> {
        let mut removed_self = Vec::new();
        let mut removed_enemy = Vec::new();
        let mut added = Vec::new();
        let mut replaced = Vec::new();
        let mut bits = mask;
        while bits != 0 {
            let sq = bits.trailing_zeros() as u8;
            bits &= bits - 1;
            let before = previous & bit(sq) != 0;
            let after = state & bit(sq) != 0;
            match (before, after) {
                (true, false) => {
                    if let Some((color, _)) = board.piece_at(sq) {
                        if color == board.side_to_move() {
                            removed_self.push(sq);
                        } else {
                            removed_enemy.push(sq);
                        }
                    } else {
                        return Err(EngineError::InvalidMask(
                            "mask referenced empty square".into(),
                        ));
                    }
                }
                (false, true) => added.push(sq),
                (true, true) => replaced.push(sq),
                (false, false) => {}
            }
        }
        Ok(Self {
            removed_self,
            removed_enemy,
            added,
            replaced,
        })
    }

    pub(crate) fn represents_move(&self) -> bool {
        !self.removed_self.is_empty()
    }

    pub(crate) fn to_intent(&self, board: &Board) -> Result<MoveIntent, EngineError> {
        if self.removed_self.len() == 2
            && self.added.len() == 2
            && self.removed_enemy.is_empty()
            && self.replaced.is_empty()
        {
            return self.castle_intent(board);
        }
        if self.removed_enemy.len() > 1 || self.replaced.len() > 1 {
            return Err(EngineError::InvalidMask(
                "too many squares changed".into(),
            ));
        }
        if self.removed_self.len() != 1 {
            return Err(EngineError::InvalidMask(
                "expected a single moving piece".into(),
            ));
        }
        let from = self.removed_self[0];
        if self.replaced.len() == 1 && self.added.is_empty() {
            let to = self.replaced[0];
            return Ok(MoveIntent::Standard {
                from,
                to,
                capture_square: None,
            });
        }
        if self.added.len() == 1 {
            let to = self.added[0];
            let capture_square = if self.removed_enemy.len() == 1 {
                Some(self.removed_enemy[0])
            } else {
                None
            };
            return Ok(MoveIntent::Standard {
                from,
                to,
                capture_square,
            });
        }
        Err(EngineError::InvalidMask("unrecognized move pattern".into()))
    }

    fn castle_intent(&self, board: &Board) -> Result<MoveIntent, EngineError> {
        let king_square = board
            .removed_king_square(&self.removed_self)
            .ok_or_else(|| EngineError::InvalidMask("king missing for castle".into()))?;
        let to = board
            .added_king_target(king_square, &self.added)
            .ok_or_else(|| EngineError::InvalidMask("castle destination not found".into()))?;
        let side = match (king_square, to) {
            (4, 6) | (60, 62) => CastleSide::KingSide,
            (4, 2) | (60, 58) => CastleSide::QueenSide,
            _ => {
                return Err(EngineError::InvalidMask(
                    "invalid castling squares".into(),
                ))
            }
        };
        Ok(MoveIntent::Castle(side))
    }
}
