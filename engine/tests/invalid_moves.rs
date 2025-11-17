use engine::{Engine, EngineError};

fn bit_for(coord: &str) -> u64 {
    let idx = Engine::square_from_coord(coord).expect("valid coordinate");
    1u64 << idx
}

fn simple_move(prev: u64, from: &str, to: &str) -> (u64, u64) {
    let from_bit = bit_for(from);
    let to_bit = bit_for(to);
    let next = (prev & !from_bit) | to_bit;
    let mut mask = prev ^ next;
    mask |= from_bit | to_bit;
    (mask, next)
}

fn move_with_capture(prev: u64, from: &str, to: &str, removed: &[&str]) -> (u64, u64) {
    let from_bit = bit_for(from);
    let mut next = prev & !from_bit;
    let to_bit = bit_for(to);
    next |= to_bit;
    for square in removed {
        next &= !bit_for(square);
    }
    let mut mask = prev ^ next;
    mask |= from_bit | to_bit;
    for square in removed {
        mask |= bit_for(square);
    }
    (mask, next)
}

#[test]
fn rejects_illegal_knight_move() {
    let mut engine = Engine::new();
    let prev_state = engine.occupancy_mask();
    let (mask, next_state) = simple_move(prev_state, "g1", "g3");
    match engine.observe(mask, next_state) {
        Err(EngineError::IllegalMove(_)) => {}
        other => panic!("expected illegal move error, got {:?}", other),
    }
}

#[test]
fn rejects_fake_en_passant() {
    let fen = "8/8/8/3pP3/8/8/8/4K3 w - - 0 1";
    let mut engine = Engine::from_fen(fen).expect("valid fen");
    let prev_state = engine.occupancy_mask();
    let (mask, next_state) = move_with_capture(prev_state, "e5", "d6", &["d5"]);
    match engine.observe(mask, next_state) {
        Err(EngineError::IllegalMove(_)) => {}
        other => panic!("expected en passant rejection, got {:?}", other),
    }
}

#[test]
fn rejects_move_exposing_king() {
    let fen = "k3r3/8/8/8/8/8/4R3/4K3 w - - 0 1";
    let mut engine = Engine::from_fen(fen).expect("valid fen");
    let prev_state = engine.occupancy_mask();
    let (mask, next_state) = simple_move(prev_state, "e2", "d2");
    match engine.observe(mask, next_state) {
        Err(EngineError::IllegalMove(_)) => {}
        other => panic!("expected king safety violation, got {:?}", other),
    }
}
