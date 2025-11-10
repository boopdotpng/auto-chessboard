use chessengine::{ChessEngine, MoveEvent};
use std::process;

const LONG_GAMES: &str = include_str!("long_games.txt");

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let games = parse_games(LONG_GAMES);
    if games.is_empty() {
        return Err("long_games.txt must contain at least one game section".into());
    }

    let mut total_positions = 0usize;
    for (game_idx, game_lines) in games.iter().enumerate() {
        let game_number = game_idx + 1;
        if game_lines.len() % 2 != 0 {
            return Err(format!("game {game_number} must have move/FEN pairs"));
        }

        let mut engine = ChessEngine::new();
        for (pair_idx, pair) in game_lines.chunks(2).enumerate() {
            let move_index = pair_idx + 1;
            let move_text = &pair[0];
            let fen = &pair[1];
            let event = MoveEvent::from_text(move_text)
                .map_err(|err| format!("game {game_number} move '{move_text}' parse error: {err}"))?;
            engine
                .apply_event(event)
                .map_err(|err| format!("game {game_number} move '{move_text}' failed: {err}"))?;
            let actual_fen = engine.to_fen();
            if actual_fen != *fen {
                return Err(format!(
                    "game {game_number} mismatch after move {move_index} ({move_text}):\n  expected: {fen}\n  actual:   {actual_fen}"
                ));
            }
            total_positions += 1;
        }
    }

    println!(
        "long_games.txt verification passed: {} game(s), {} position(s) checked.",
        games.len(),
        total_positions
    );
    Ok(())
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
