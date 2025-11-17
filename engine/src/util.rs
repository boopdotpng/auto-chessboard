use crate::types::Bitboard;

pub(crate) fn bit(square: u8) -> Bitboard {
    1u64 << square
}

pub(crate) fn coord_to_square(coord: &str) -> Option<u8> {
    if coord.len() != 2 {
        return None;
    }
    let mut chars = coord.chars();
    let file = chars.next()?.to_ascii_lowercase();
    let rank = chars.next()?;
    if !('a'..='h').contains(&file) || !('1'..='8').contains(&rank) {
        return None;
    }
    let file_idx = (file as u8 - b'a') as u8;
    let rank_idx = (rank as u8 - b'1') as u8;
    Some(rank_idx * 8 + file_idx)
}

pub(crate) fn square_to_coord(square: u8) -> String {
    let file = (square % 8) as u8;
    let rank = (square / 8) as u8;
    let mut out = String::new();
    out.push((b'a' + file) as char);
    out.push((b'1' + rank) as char);
    out
}

pub(crate) fn file_of(square: u8) -> u8 {
    square % 8
}

pub(crate) fn rank_of(square: u8) -> u8 {
    square / 8
}
