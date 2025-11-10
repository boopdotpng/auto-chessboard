use chessengine::{ChessEngine, MoveEvent};

fn event(s: &str) -> MoveEvent {
    MoveEvent::from_text(s).unwrap()
}

#[test]
fn starting_position_fen() {
    let engine = ChessEngine::new();
    assert_eq!(
        engine.to_fen(),
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
    );
}

#[test]
fn apply_simple_move() {
    let mut engine = ChessEngine::new();
    engine.apply_event(event("e2-e4")).unwrap();
    assert_eq!(
        engine.to_fen(),
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1"
    );
    assert_eq!(engine.pgn(), "1.e4");
}

#[test]
fn apply_two_moves_pgn() {
    let mut engine = ChessEngine::new();
    engine.apply_event(event("e2-e4")).unwrap();
    engine.apply_event(event("e7-e5")).unwrap();
    assert_eq!(engine.pgn(), "1.e4 e5");
}
