use crate::change::MoveIntent;
use crate::types::{Bitboard, CastleSide, Color, EngineError, Move, PieceKind};
use crate::util::{bit, coord_to_square, file_of, rank_of, square_to_coord};

#[derive(Clone)]
pub(crate) struct Board {
    pieces: [[Bitboard; 6]; 2],
    side_to_move: Color,
    castling: u8,
    en_passant: Option<u8>,
    halfmove_clock: u32,
    fullmove_number: u32,
}

impl Board {
    pub(crate) fn starting_position() -> Self {
        Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap()
    }

    pub(crate) fn occupancy(&self) -> Bitboard {
        self.pieces[0].iter().fold(0, |acc, bb| acc | bb)
            | self.pieces[1].iter().fold(0, |acc, bb| acc | bb)
    }

    pub(crate) fn piece_at(&self, square: u8) -> Option<(Color, PieceKind)> {
        for color in [Color::White, Color::Black] {
            for (idx, bb) in self.pieces[color.idx()].iter().enumerate() {
                if bb & bit(square) != 0 {
                    return Some((color, PieceKind::ALL[idx]));
                }
            }
        }
        None
    }

    pub(crate) fn promote_piece(&mut self, square: u8, new_piece: PieceKind) -> Result<(), EngineError> {
        let (color, piece) = self
            .piece_at(square)
            .ok_or_else(|| EngineError::IllegalMove("promotion square empty".into()))?;
        if piece != PieceKind::Pawn {
            return Err(EngineError::IllegalMove(
                "promotion square does not contain a pawn".into(),
            ));
        }
        self.pieces[color.idx()][kind_idx(PieceKind::Pawn)] &= !bit(square);
        self.pieces[color.idx()][kind_idx(new_piece)] |= bit(square);
        Ok(())
    }

    pub(crate) fn from_fen(fen: &str) -> Result<Self, EngineError> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() != 6 {
            return Err(EngineError::InvalidFen(
                "FEN must have 6 space-separated fields".into(),
            ));
        }

        let mut pieces = [[0u64; 6]; 2];
        let mut rank = 7i32;
        let mut file = 0i32;
        for ch in parts[0].chars() {
            match ch {
                '/' => {
                    if file != 8 {
                        return Err(EngineError::InvalidFen(
                            "rank does not contain 8 squares".into(),
                        ));
                    }
                    rank -= 1;
                    file = 0;
                }
                '1'..='8' => {
                    file += ch.to_digit(10).unwrap() as i32;
                    if file > 8 {
                        return Err(EngineError::InvalidFen(
                            "too many squares in rank".into(),
                        ));
                    }
                }
                _ => {
                    let color = Color::from_char(ch).ok_or_else(|| {
                        EngineError::InvalidFen(format!("invalid piece char '{ch}'"))
                    })?;
                    let piece = match ch.to_ascii_lowercase() {
                        'p' => PieceKind::Pawn,
                        'n' => PieceKind::Knight,
                        'b' => PieceKind::Bishop,
                        'r' => PieceKind::Rook,
                        'q' => PieceKind::Queen,
                        'k' => PieceKind::King,
                        _ => unreachable!(),
                    };
                    if !(0..8).contains(&rank) || !(0..8).contains(&file) {
                        return Err(EngineError::InvalidFen("square out of range".into()));
                    }
                    let square = (rank * 8 + file) as u8;
                    pieces[color.idx()][kind_idx(piece)] |= bit(square);
                    file += 1;
                }
            }
        }
        if rank != 0 || file != 8 {
            return Err(EngineError::InvalidFen("invalid board layout".into()));
        }

        let side_to_move = match parts[1] {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return Err(EngineError::InvalidFen("invalid side to move".into())),
        };

        let mut castling = 0u8;
        if parts[2] != "-" {
            for ch in parts[2].chars() {
                castling |= match ch {
                    'K' => CastleSide::KingSide.as_bits(Color::White),
                    'Q' => CastleSide::QueenSide.as_bits(Color::White),
                    'k' => CastleSide::KingSide.as_bits(Color::Black),
                    'q' => CastleSide::QueenSide.as_bits(Color::Black),
                    _ => {
                        return Err(EngineError::InvalidFen(
                            "invalid castling rights".into(),
                        ))
                    }
                };
            }
        }

        let en_passant = if parts[3] == "-" {
            None
        } else {
            Some(
                coord_to_square(parts[3])
                    .ok_or_else(|| EngineError::InvalidFen("invalid en passant".into()))?,
            )
        };

        let halfmove_clock = parts[4]
            .parse::<u32>()
            .map_err(|_| EngineError::InvalidFen("invalid halfmove".into()))?;
        let fullmove_number = parts[5]
            .parse::<u32>()
            .map_err(|_| EngineError::InvalidFen("invalid fullmove".into()))?;

        Ok(Self {
            pieces,
            side_to_move,
            castling,
            en_passant,
            halfmove_clock,
            fullmove_number,
        })
    }

    pub(crate) fn to_fen(&self) -> String {
        let mut rows = Vec::new();
        for rank in (0..8).rev() {
            let mut row = String::new();
            let mut empty = 0;
            for file in 0..8 {
                let square = rank * 8 + file;
                if let Some((color, piece)) = self.piece_at(square as u8) {
                    if empty > 0 {
                        row.push_str(&empty.to_string());
                        empty = 0;
                    }
                    row.push(piece.fen_symbol(color));
                } else {
                    empty += 1;
                }
            }
            if empty > 0 {
                row.push_str(&empty.to_string());
            }
            rows.push(row);
        }

        let board = rows.join("/");
        let stm = match self.side_to_move {
            Color::White => 'w',
            Color::Black => 'b',
        };
        let castle = if self.castling == 0 {
            "-".to_string()
        } else {
            let mut rights = String::new();
            if self.castling & CastleSide::KingSide.as_bits(Color::White) != 0 {
                rights.push('K');
            }
            if self.castling & CastleSide::QueenSide.as_bits(Color::White) != 0 {
                rights.push('Q');
            }
            if self.castling & CastleSide::KingSide.as_bits(Color::Black) != 0 {
                rights.push('k');
            }
            if self.castling & CastleSide::QueenSide.as_bits(Color::Black) != 0 {
                rights.push('q');
            }
            rights
        };
        let enp = self
            .en_passant
            .map(square_to_coord)
            .unwrap_or_else(|| "-".to_string());
        format!(
            "{board} {stm} {castle} {enp} {} {}",
            self.halfmove_clock, self.fullmove_number
        )
    }

    pub(crate) fn apply_move(&mut self, mv: &Move) {
        self.en_passant = None;
        let from_bit = bit(mv.from);
        let to_bit = bit(mv.to);
        self.pieces[mv.color.idx()][kind_idx(mv.piece)] &= !from_bit;
        match mv.castle {
            Some(CastleSide::KingSide) => {
                self.pieces[mv.color.idx()][kind_idx(PieceKind::King)] |= to_bit;
                let (rook_from, rook_to) = match mv.color {
                    Color::White => (7, 5),
                    Color::Black => (63, 61),
                };
                self.pieces[mv.color.idx()][kind_idx(PieceKind::Rook)] &= !bit(rook_from);
                self.pieces[mv.color.idx()][kind_idx(PieceKind::Rook)] |= bit(rook_to);
                self.disable_castling(mv.color);
            }
            Some(CastleSide::QueenSide) => {
                self.pieces[mv.color.idx()][kind_idx(PieceKind::King)] |= to_bit;
                let (rook_from, rook_to) = match mv.color {
                    Color::White => (0, 3),
                    Color::Black => (56, 59),
                };
                self.pieces[mv.color.idx()][kind_idx(PieceKind::Rook)] &= !bit(rook_from);
                self.pieces[mv.color.idx()][kind_idx(PieceKind::Rook)] |= bit(rook_to);
                self.disable_castling(mv.color);
            }
            None => {
                if let Some(captured) = mv.capture {
                    let capture_sq = mv.capture_square.unwrap_or(mv.to);
                    self.pieces[mv.color.opponent().idx()][kind_idx(captured)] &= !bit(capture_sq);
                }
                self.pieces[mv.color.idx()][kind_idx(mv.piece)] |= to_bit;
                if mv.piece == PieceKind::King {
                    self.disable_castling(mv.color);
                } else if mv.piece == PieceKind::Rook {
                    self.remove_castling_rights(mv.color, mv.from);
                }
                if let Some(capture_sq) = mv.capture_square {
                    self.remove_castling_rights(mv.color.opponent(), capture_sq);
                }
            }
        }
        if mv.is_double_pawn_push {
            if let Some(ep) = self.double_push_target(mv) {
                self.en_passant = Some(ep);
            }
        }
        if self.side_to_move == Color::Black {
            self.fullmove_number += 1;
        }
        self.side_to_move = self.side_to_move.opponent();
        self.halfmove_clock = if mv.piece == PieceKind::Pawn || mv.capture.is_some() {
            0
        } else {
            self.halfmove_clock + 1
        };
    }

    pub(crate) fn validate_intent(&self, intent: MoveIntent) -> Result<Move, EngineError> {
        match intent {
            MoveIntent::Castle(side) => self.validate_castle(side),
            MoveIntent::Standard {
                from,
                to,
                capture_square,
            } => self.validate_standard_move(from, to, capture_square),
        }
    }

    pub(crate) fn side_to_move(&self) -> Color {
        self.side_to_move
    }

    pub(crate) fn removed_king_square(&self, squares: &[u8]) -> Option<u8> {
        squares.iter().copied().find(|sq| {
            self.pieces[self.side_to_move.idx()][kind_idx(PieceKind::King)] & bit(*sq) != 0
        })
    }

    pub(crate) fn added_king_target(&self, from: u8, added: &[u8]) -> Option<u8> {
        let targets = match (self.side_to_move, from) {
            (Color::White, 4) => [6, 2],
            (Color::Black, 60) => [62, 58],
            _ => return None,
        };
        added.iter().copied().find(|sq| targets.contains(sq))
    }

    fn disable_castling(&mut self, color: Color) {
        match color {
            Color::White => {
                self.castling &= !(CastleSide::KingSide.as_bits(Color::White)
                    | CastleSide::QueenSide.as_bits(Color::White))
            }
            Color::Black => {
                self.castling &= !(CastleSide::KingSide.as_bits(Color::Black)
                    | CastleSide::QueenSide.as_bits(Color::Black))
            }
        }
    }

    fn remove_castling_rights(&mut self, color: Color, square: u8) {
        match (color, square) {
            (Color::White, 0) => self.castling &= !CastleSide::QueenSide.as_bits(Color::White),
            (Color::White, 7) => self.castling &= !CastleSide::KingSide.as_bits(Color::White),
            (Color::Black, 56) => self.castling &= !CastleSide::QueenSide.as_bits(Color::Black),
            (Color::Black, 63) => self.castling &= !CastleSide::KingSide.as_bits(Color::Black),
            _ => {}
        }
    }

    fn double_push_target(&self, mv: &Move) -> Option<u8> {
        if mv.piece != PieceKind::Pawn || !mv.is_double_pawn_push {
            return None;
        }
        let target = if mv.color == Color::White {
            mv.from + 8
        } else {
            mv.from - 8
        };
        let rank = rank_of(mv.to);
        let file = file_of(mv.to) as i8;
        let opponent_pawns = self.pieces[mv.color.opponent().idx()][kind_idx(PieceKind::Pawn)];
        for df in [-1, 1] {
            let nf = file + df;
            if nf < 0 || nf >= 8 {
                continue;
            }
            let sq = rank * 8 + nf as u8;
            if opponent_pawns & bit(sq) != 0 {
                return Some(target);
            }
        }
        None
    }

    fn validate_castle(&self, side: CastleSide) -> Result<Move, EngineError> {
        let color = self.side_to_move;
        let rights = side.as_bits(color);
        if self.castling & rights == 0 {
            return Err(EngineError::IllegalMove("castling not permitted".into()));
        }
        let (king_from, king_to, rook_from, _rook_to, between, attack_squares): (
            u8,
            u8,
            u8,
            u8,
            &[u8],
            &[u8],
        ) = match (color, side) {
            (Color::White, CastleSide::KingSide) => (4, 6, 7, 5, &[5, 6], &[4, 5, 6]),
            (Color::White, CastleSide::QueenSide) => (4, 2, 0, 3, &[1, 2, 3], &[4, 3, 2]),
            (Color::Black, CastleSide::KingSide) => (60, 62, 63, 61, &[61, 62], &[60, 61, 62]),
            (Color::Black, CastleSide::QueenSide) => (60, 58, 56, 59, &[57, 58, 59], &[60, 59, 58]),
        };
        if self.pieces[color.idx()][kind_idx(PieceKind::King)] & bit(king_from) == 0 {
            return Err(EngineError::IllegalMove("king not on expected square".into()));
        }
        if self.pieces[color.idx()][kind_idx(PieceKind::Rook)] & bit(rook_from) == 0 {
            return Err(EngineError::IllegalMove("rook missing for castling".into()));
        }
        for &sq in between {
            if self.occupancy() & bit(sq) != 0 {
                return Err(EngineError::IllegalMove("squares blocked".into()));
            }
        }
        for &sq in attack_squares {
            if self.is_square_attacked(sq, color.opponent()) {
                return Err(EngineError::IllegalMove(
                    "cannot castle through check".into(),
                ));
            }
        }
        Ok(Move {
            color,
            piece: PieceKind::King,
            from: king_from,
            to: king_to,
            capture: None,
            capture_square: None,
            castle: Some(side),
            is_en_passant: false,
            is_double_pawn_push: false,
            promotion: None,
            requires_promotion: false,
        })
    }

    fn validate_standard_move(
        &self,
        from: u8,
        to: u8,
        capture_square: Option<u8>,
    ) -> Result<Move, EngineError> {
        let (color, piece) = self
            .piece_at(from)
            .ok_or_else(|| EngineError::IllegalMove("no piece on from square".into()))?;
        if color != self.side_to_move {
            return Err(EngineError::IllegalMove("wrong side to move".into()));
        }

        if let Some((dest_color, _)) = self.piece_at(to) {
            if dest_color == color {
                return Err(EngineError::IllegalMove(
                    "destination occupied by friendly piece".into(),
                ));
            }
        }

        let mut mv = Move {
            color,
            piece,
            from,
            to,
            capture: None,
            capture_square,
            castle: None,
            is_en_passant: false,
            is_double_pawn_push: false,
            promotion: None,
            requires_promotion: false,
        };

        match piece {
            PieceKind::Pawn => self.validate_pawn_move(&mut mv)?,
            PieceKind::Knight => self.validate_knight_path(from, to)?,
            PieceKind::Bishop => self.validate_bishop_path(from, to)?,
            PieceKind::Rook => self.validate_rook_path(from, to)?,
            PieceKind::Queen => {
                if self.is_diagonal_move(from, to) {
                    self.validate_bishop_path(from, to)?;
                } else if self.is_straight_move(from, to) {
                    self.validate_rook_path(from, to)?;
                } else {
                    return Err(EngineError::IllegalMove("invalid queen move".into()));
                }
            }
            PieceKind::King => self.validate_king_path(from, to)?,
        }

        if piece != PieceKind::Pawn {
            if let Some((target_color, target_piece)) = self.piece_at(to) {
                if target_color == color {
                    return Err(EngineError::IllegalMove("destination occupied".into()));
                }
                mv.capture = Some(target_piece);
            } else if mv.capture_square.is_some() {
                return Err(EngineError::IllegalMove("capture square empty".into()));
            }
        }

        let mut clone = self.clone();
        clone.apply_move(&mv);
        if clone.is_square_attacked(clone.king_square(color).unwrap(), color.opponent()) {
            return Err(EngineError::IllegalMove("king would be in check".into()));
        }

        Ok(mv)
    }

    fn validate_pawn_move(&self, mv: &mut Move) -> Result<(), EngineError> {
        let dir: i32 = if mv.color == Color::White { 8 } else { -8 };
        let from_rank = rank_of(mv.from);
        let to_rank = rank_of(mv.to);
        let forward_one = (mv.from as i32 + dir) as u8;
        if mv.to == forward_one {
            if mv.capture_square.is_some() {
                return Err(EngineError::IllegalMove(
                    "capture square provided for quiet move".into(),
                ));
            }
            if self.occupancy() & bit(mv.to) != 0 {
                return Err(EngineError::IllegalMove("square occupied".into()));
            }
        } else if mv.to == (mv.from as i32 + dir * 2) as u8 {
            let start_rank = if mv.color == Color::White { 1 } else { 6 };
            if from_rank != start_rank {
                return Err(EngineError::IllegalMove(
                    "double push only from starting rank".into(),
                ));
            }
            if mv.capture_square.is_some() {
                return Err(EngineError::IllegalMove(
                    "double push cannot capture".into(),
                ));
            }
            if self.occupancy() & bit((mv.from as i32 + dir) as u8) != 0
                || self.occupancy() & bit(mv.to) != 0
            {
                return Err(EngineError::IllegalMove("path blocked".into()));
            }
            mv.is_double_pawn_push = true;
        } else {
            let file_diff = file_of(mv.to) as i8 - file_of(mv.from) as i8;
            if file_diff.abs() != 1
                || (to_rank as i32 - from_rank as i32)
                    != if mv.color == Color::White { 1 } else { -1 }
            {
                return Err(EngineError::IllegalMove("invalid pawn capture".into()));
            }
            let capture_sq = mv.capture_square.unwrap_or(mv.to);
            if let Some((target_color, target_piece)) = self.piece_at(capture_sq) {
                if target_color == mv.color {
                    return Err(EngineError::IllegalMove("cannot capture own piece".into()));
                }
                mv.capture = Some(target_piece);
                if mv.capture_square.is_some() && target_piece != PieceKind::Pawn {
                    return Err(EngineError::IllegalMove(
                        "en passant must capture pawn".into(),
                    ));
                }
                if mv.capture_square.is_some() && self.en_passant != Some(mv.to) {
                    return Err(EngineError::IllegalMove(
                        "en passant target not available".into(),
                    ));
                }
                mv.is_en_passant = mv.capture_square.is_some();
            } else {
                return Err(EngineError::IllegalMove("missing capture target".into()));
            }
        }
        let promotion_rank = if mv.color == Color::White { 7 } else { 0 };
        if to_rank == promotion_rank {
            mv.requires_promotion = true;
        }
        Ok(())
    }

    fn validate_knight_path(&self, from: u8, to: u8) -> Result<(), EngineError> {
        let file_diff = (file_of(to) as i32 - file_of(from) as i32).abs();
        let rank_diff = (rank_of(to) as i32 - rank_of(from) as i32).abs();
        if (file_diff == 1 && rank_diff == 2) || (file_diff == 2 && rank_diff == 1) {
            Ok(())
        } else {
            Err(EngineError::IllegalMove("invalid knight move".into()))
        }
    }

    fn validate_rook_path(&self, from: u8, to: u8) -> Result<(), EngineError> {
        if !self.is_straight_move(from, to) {
            return Err(EngineError::IllegalMove("rook moves straight".into()));
        }
        if self.path_blocked(from, to) {
            return Err(EngineError::IllegalMove("path blocked".into()));
        }
        Ok(())
    }

    fn validate_bishop_path(&self, from: u8, to: u8) -> Result<(), EngineError> {
        if !self.is_diagonal_move(from, to) {
            return Err(EngineError::IllegalMove("bishop moves diagonal".into()));
        }
        if self.path_blocked(from, to) {
            return Err(EngineError::IllegalMove("path blocked".into()));
        }
        Ok(())
    }

    fn validate_king_path(&self, from: u8, to: u8) -> Result<(), EngineError> {
        let file_diff = (file_of(to) as i32 - file_of(from) as i32).abs();
        let rank_diff = (rank_of(to) as i32 - rank_of(from) as i32).abs();
        if file_diff <= 1 && rank_diff <= 1 {
            Ok(())
        } else {
            Err(EngineError::IllegalMove("invalid king move".into()))
        }
    }

    fn path_blocked(&self, from: u8, to: u8) -> bool {
        let from_file = file_of(from) as i32;
        let from_rank = rank_of(from) as i32;
        let to_file = file_of(to) as i32;
        let to_rank = rank_of(to) as i32;
        let file_step = (to_file - from_file).signum();
        let rank_step = (to_rank - from_rank).signum();
        let mut file = from_file + file_step;
        let mut rank = from_rank + rank_step;
        while file != to_file || rank != to_rank {
            let sq = (rank * 8 + file) as u8;
            if self.occupancy() & bit(sq) != 0 {
                return true;
            }
            file += file_step;
            rank += rank_step;
        }
        false
    }

    fn is_straight_move(&self, from: u8, to: u8) -> bool {
        file_of(from) == file_of(to) || rank_of(from) == rank_of(to)
    }

    fn is_diagonal_move(&self, from: u8, to: u8) -> bool {
        (file_of(from) as i32 - file_of(to) as i32).abs()
            == (rank_of(from) as i32 - rank_of(to) as i32).abs()
    }

    fn king_square(&self, color: Color) -> Option<u8> {
        let bb = self.pieces[color.idx()][kind_idx(PieceKind::King)];
        if bb == 0 {
            return None;
        }
        Some(bb.trailing_zeros() as u8)
    }

    fn is_square_attacked(&self, square: u8, by: Color) -> bool {
        let rank = rank_of(square);
        let file = file_of(square);
        let pawn_dirs: &[(i32, i32)] = match by {
            Color::White => &[(-1, -1), (1, -1)],
            Color::Black => &[(-1, 1), (1, 1)],
        };
        for (df, dr) in pawn_dirs {
            let nf = file as i32 + df;
            let nr = rank as i32 + dr;
            if (0..8).contains(&nf) && (0..8).contains(&nr) {
                let sq = (nr * 8 + nf) as u8;
                if self.pieces[by.idx()][kind_idx(PieceKind::Pawn)] & bit(sq) != 0 {
                    return true;
                }
            }
        }

        for (df, dr) in KNIGHT_DELTAS {
            let nf = file as i32 + df;
            let nr = rank as i32 + dr;
            if (0..8).contains(&nf) && (0..8).contains(&nr) {
                let sq = (nr * 8 + nf) as u8;
                if self.pieces[by.idx()][kind_idx(PieceKind::Knight)] & bit(sq) != 0 {
                    return true;
                }
            }
        }

        for &(df, dr) in DIAGONAL_DELTAS.iter() {
            if self.scan_ray(square, df, dr, by, PieceKind::Bishop) {
                return true;
            }
        }
        for &(df, dr) in ORTHO_DELTAS.iter() {
            if self.scan_ray(square, df, dr, by, PieceKind::Rook) {
                return true;
            }
        }

        for df in -1..=1 {
            for dr in -1..=1 {
                if df == 0 && dr == 0 {
                    continue;
                }
                let nf = file as i32 + df;
                let nr = rank as i32 + dr;
                if (0..8).contains(&nf) && (0..8).contains(&nr) {
                    let sq = (nr * 8 + nf) as u8;
                    if self.pieces[by.idx()][kind_idx(PieceKind::King)] & bit(sq) != 0 {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn scan_ray(
        &self,
        start: u8,
        df: i32,
        dr: i32,
        color: Color,
        major: PieceKind,
    ) -> bool {
        let mut file = file_of(start) as i32 + df;
        let mut rank = rank_of(start) as i32 + dr;
        while (0..8).contains(&file) && (0..8).contains(&rank) {
            let sq = (rank * 8 + file) as u8;
            if self.occupancy() & bit(sq) != 0 {
                if let Some((c, piece)) = self.piece_at(sq) {
                    if c == color && (piece == PieceKind::Queen || piece == major) {
                        return true;
                    }
                }
                return false;
            }
            file += df;
            rank += dr;
        }
        false
    }
}

const KNIGHT_DELTAS: [(i32, i32); 8] = [
    (1, 2),
    (2, 1),
    (2, -1),
    (1, -2),
    (-1, -2),
    (-2, -1),
    (-2, 1),
    (-1, 2),
];

const DIAGONAL_DELTAS: [(i32, i32); 4] = [
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
];

const ORTHO_DELTAS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

fn kind_idx(kind: PieceKind) -> usize {
    match kind {
        PieceKind::Pawn => 0,
        PieceKind::Knight => 1,
        PieceKind::Bishop => 2,
        PieceKind::Rook => 3,
        PieceKind::Queen => 4,
        PieceKind::King => 5,
    }
}
