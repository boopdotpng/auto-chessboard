use engine::{Engine, EngineError, EngineUpdate, PieceKind};

const LONG_GAMES: &str = include_str!("long_games.txt");
const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

#[test]
fn long_games_replay_matches_fen() {
    let games = parse_games(LONG_GAMES);
    assert!(!games.is_empty(), "long_games.txt must contain games");

    for (game_idx, game) in games.iter().enumerate() {
        assert!(game.len() % 2 == 0, "game {} must have move/FEN pairs", game_idx + 1);
        let mut engine = Engine::new();
        let mut prev_state = Engine::from_fen(START_FEN)
            .expect("start FEN invalid")
            .occupancy_mask();
        for (pair_idx, chunk) in game.chunks(2).enumerate() {
            let move_text = chunk[0].trim();
            let expected_fen = chunk[1].trim();
            let next_state = Engine::from_fen(expected_fen)
                .expect("invalid expected FEN")
                .occupancy_mask();
            let mask = compute_mask(prev_state, next_state, move_text)
                .unwrap_or_else(|err| panic!(
                    "game {} move {} ({}): mask error: {}",
                    game_idx + 1,
                    pair_idx + 1,
                    move_text,
                    err
                ));
            let update = engine.observe(mask, next_state).unwrap_or_else(|err| {
                let diff = squares_from_bits(prev_state ^ next_state);
                let mask_bits = squares_from_bits(mask);
                panic!(
                    "game {} move {} ({}): {}. diff={:?} mask={:?}",
                    game_idx + 1,
                    pair_idx + 1,
                    move_text,
                    err,
                    diff,
                    mask_bits
                )
            });
            if let Some(promo) = promotion_piece(move_text) {
                match update {
                    EngineUpdate::PromotionPending(_) => {
                        let summary = engine
                            .confirm_promotion(promo)
                            .unwrap_or_else(|err| panic!(
                                "game {} move {} ({}): {}",
                                game_idx + 1,
                                pair_idx + 1,
                                move_text,
                                err
                            ));
                        assert_eq!(
                            summary.fen, expected_fen,
                            "game {} move {} ({}): FEN mismatch after promotion",
                            game_idx + 1,
                            pair_idx + 1,
                            move_text
                        );
                    }
                    other => panic!(
                        "game {} move {} ({}): expected promotion pending, got {:?}",
                        game_idx + 1,
                        pair_idx + 1,
                        move_text,
                        other
                    ),
                }
            } else {
                match update {
                    EngineUpdate::MoveApplied(summary) => assert_eq!(
                        summary.fen, expected_fen,
                        "game {} move {} ({}): FEN mismatch",
                        game_idx + 1,
                        pair_idx + 1,
                        move_text
                    ),
                    EngineUpdate::NoChange => panic!(
                        "game {} move {} ({}): observation produced no change",
                        game_idx + 1,
                        pair_idx + 1,
                        move_text
                    ),
                    EngineUpdate::PromotionPending(_) => panic!(
                        "game {} move {} ({}): unexpected promotion",
                        game_idx + 1,
                        pair_idx + 1,
                        move_text
                    ),
                }
            }
            prev_state = next_state;
        }
    }
}

fn parse_games(input: &str) -> Vec<Vec<String>> {
    let mut games = Vec::new();
    let mut current = Vec::new();
    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "---" {
            if !current.is_empty() {
                games.push(current);
                current = Vec::new();
            }
            continue;
        }
        current.push(trimmed.to_string());
    }
    if !current.is_empty() {
        games.push(current);
    }
    games
}

fn compute_mask(prev_state: u64, next_state: u64, move_text: &str) -> Result<u64, EngineError> {
    let mut mask = prev_state ^ next_state;
    for segment in move_text.split(',') {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        let (coord_part, _) = segment.split_once('=').unwrap_or((segment, ""));
        let (from, to) = coord_part
            .split_once('-')
            .ok_or_else(|| EngineError::IllegalMove(format!("invalid move segment '{segment}'")))?;
        let from_sq = Engine::square_from_coord(from)?;
        let to_sq = Engine::square_from_coord(to)?;
        mask |= (1u64 << from_sq) | (1u64 << to_sq);
    }
    Ok(mask)
}

fn promotion_piece(move_text: &str) -> Option<PieceKind> {
    move_text.split(',').find_map(|segment| {
        let (_, tail) = segment.split_once('=')?;
        let ch = tail.chars().next()?;
        match ch {
            'Q' => Some(PieceKind::Queen),
            'R' => Some(PieceKind::Rook),
            'B' => Some(PieceKind::Bishop),
            'N' => Some(PieceKind::Knight),
            _ => None,
        }
    })
}

fn squares_from_bits(mut bb: u64) -> Vec<String> {
    let mut out = Vec::new();
    while bb != 0 {
        let idx = bb.trailing_zeros() as u8;
        out.push(square_name(idx));
        bb &= bb - 1;
    }
    out
}

fn square_name(idx: u8) -> String {
    let file = (idx % 8) as u8;
    let rank = (idx / 8) as u8;
    let mut s = String::new();
    s.push((b'a' + file) as char);
    s.push((b'1' + rank) as char);
    s
}
