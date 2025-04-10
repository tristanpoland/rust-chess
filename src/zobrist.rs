use crate::piece::{PieceType, Color};
use rand::prelude::*;

pub const WHITE: usize = 0;
pub const BLACK: usize = 1;
pub const PAWN: usize = 0;
pub const KNIGHT: usize = 1;
pub const BISHOP: usize = 2;
pub const ROOK: usize = 3;
pub const QUEEN: usize = 4;
pub const KING: usize = 5;

pub struct ZobristKeys {
    pub piece_keys: [[[u64; 64]; 6]; 2],
    pub castling_keys: [u64; 4],
    pub en_passant_keys: [u64; 8],
    pub side_to_move_key: u64,
}

impl ZobristKeys {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        
        let mut piece_keys = [[[0; 64]; 6]; 2];
        for color in 0..2 {
            for piece_type in 0..6 {
                for square in 0..64 {
                    piece_keys[color][piece_type][square] = rng.gen::<u64>();
                }
            }
        }
        
        let mut castling_keys = [0; 4];
        for i in 0..4 {
            castling_keys[i] = rng.gen::<u64>();
        }
        
        let mut en_passant_keys = [0; 8];
        for i in 0..8 {
            en_passant_keys[i] = rng.gen::<u64>();
        }
        
        let side_to_move_key = rng.gen::<u64>();
        
        Self {
            piece_keys,
            castling_keys,
            en_passant_keys,
            side_to_move_key,
        }
    }
    
    pub fn get_piece_index(piece_type: PieceType) -> usize {
        match piece_type {
            PieceType::Pawn => PAWN,
            PieceType::Knight => KNIGHT,
            PieceType::Bishop => BISHOP,
            PieceType::Rook => ROOK,
            PieceType::Queen => QUEEN,
            PieceType::King => KING,
        }
    }
    
    pub fn get_color_index(color: Color) -> usize {
        match color {
            Color::White => WHITE,
            Color::Black => BLACK,
        }
    }
}

lazy_static::lazy_static! {
    pub static ref ZOBRIST: ZobristKeys = ZobristKeys::new();
} 
