use crate::piece::{Piece, PieceType, Color};
use crate::zobrist::{ZOBRIST, WHITE, BLACK};
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub const BOARD_SIZE: usize = 8;
pub type Square = Option<Piece>;
pub type Board = [[Square; BOARD_SIZE]; BOARD_SIZE];

pub struct PromotionState {
    pub position: (usize, usize),
    pub color: Color,
}

#[derive(Clone, Copy, Debug)]
pub struct Timer {
    pub initial_time: Duration,
    pub remaining: Duration,
    pub increment: Duration,
    pub last_update: Option<Instant>,
    pub is_running: bool,
}

impl Timer {
    pub fn new(minutes: u64, increment_seconds: u64) -> Self {
        Self {
            initial_time: Duration::from_secs(minutes * 60),
            remaining: Duration::from_secs(minutes * 60),
            increment: Duration::from_secs(increment_seconds),
            last_update: None,
            is_running: false,
        }
    }
    
    pub fn start(&mut self) {
        if !self.is_running {
            println!("Timer started with remaining time: {}", self.format_time());
            self.last_update = Some(Instant::now());
            self.is_running = true;
        }
    }
    
    pub fn stop(&mut self) -> bool {
        if self.is_running {
            if let Some(last_update) = self.last_update {
                let elapsed = last_update.elapsed();
                if elapsed <= self.remaining {
                    self.remaining -= elapsed;
                    self.remaining += self.increment; // Add increment
                    println!("Timer stopped at: {} (added increment: {}s)", 
                             self.format_time(), self.increment.as_secs());
                } else {
                    println!("Timer expired during stop!");
                    self.remaining = Duration::from_secs(0);
                    self.is_running = false;
                    return true; // Time flag fallen
                }
            }
            self.is_running = false;
        }
        false // Time not expired
    }
    
    pub fn update(&mut self) -> bool {
        if !self.is_running {
            return false;
        }
        
        if let Some(last_update) = self.last_update {
            let elapsed = last_update.elapsed();
            
            // Only update if some measurable time has passed
            if elapsed.as_millis() > 0 {
                // Check if time has expired
                if elapsed >= self.remaining {
                    println!("Timer expired! Elapsed: {:?}, Remaining: {:?}", 
                             elapsed, self.remaining);
                    self.remaining = Duration::from_secs(0);
                    self.is_running = false;
                    return true; // Time flag fallen
                }
                
                // Decrement the remaining time
                self.remaining = self.remaining.saturating_sub(elapsed);
                
                // Update the last_update to now to start counting from this point
                self.last_update = Some(Instant::now());
                
                // Debug output every whole second change
                if elapsed.as_secs() > 0 {
                    println!("Timer updated, remaining: {}", self.format_time());
                }
            }
        } else {
            // If last_update is None but timer is running, reset it
            println!("Timer was running but last_update was None, resetting timer");
            self.last_update = Some(Instant::now());
        }
        
        false // Time not expired
    }
    
    pub fn reset(&mut self) {
        self.remaining = self.initial_time;
        self.last_update = None;
        self.is_running = false;
    }
    
    pub fn format_time(&self) -> String {
        let total_seconds = self.remaining.as_secs();
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{:02}:{:02}", minutes, seconds)
    }
}

pub struct GameState {
    pub board: Board,
    pub current_turn: Color,
    pub white_can_castle_kingside: bool,
    pub white_can_castle_queenside: bool,
    pub black_can_castle_kingside: bool,
    pub black_can_castle_queenside: bool,
    pub en_passant_target: Option<(usize, usize)>,
    pub halfmove_clock: u32,
    pub fullmove_number: u32,
    pub promotion_pending: Option<PromotionState>,
    
    pub position_history: HashMap<u64, u32>, // Maps hash to occurrence count
    pub current_hash: u64,                  // Current position hash
    
    pub draw_offered: Option<Color>, // Tracks which player has offered a draw
    
    pub white_timer: Timer,
    pub black_timer: Timer,
    pub timer_enabled: bool,
    
    move_cache: HashMap<u64, Vec<((usize, usize), (usize, usize))>>, // Maps position hash to legal moves
}

impl GameState {
    pub fn new() -> Self {
        let mut board = [[None; BOARD_SIZE]; BOARD_SIZE];
        
        for file in 0..BOARD_SIZE {
            board[1][file] = Some(Piece::new(PieceType::Pawn, Color::Black));
            board[6][file] = Some(Piece::new(PieceType::Pawn, Color::White));
        }
        
        board[0][0] = Some(Piece::new(PieceType::Rook, Color::Black));
        board[0][1] = Some(Piece::new(PieceType::Knight, Color::Black));
        board[0][2] = Some(Piece::new(PieceType::Bishop, Color::Black));
        board[0][3] = Some(Piece::new(PieceType::Queen, Color::Black));
        board[0][4] = Some(Piece::new(PieceType::King, Color::Black));
        board[0][5] = Some(Piece::new(PieceType::Bishop, Color::Black));
        board[0][6] = Some(Piece::new(PieceType::Knight, Color::Black));
        board[0][7] = Some(Piece::new(PieceType::Rook, Color::Black));
        
        board[7][0] = Some(Piece::new(PieceType::Rook, Color::White));
        board[7][1] = Some(Piece::new(PieceType::Knight, Color::White));
        board[7][2] = Some(Piece::new(PieceType::Bishop, Color::White));
        board[7][3] = Some(Piece::new(PieceType::Queen, Color::White));
        board[7][4] = Some(Piece::new(PieceType::King, Color::White));
        board[7][5] = Some(Piece::new(PieceType::Bishop, Color::White));
        board[7][6] = Some(Piece::new(PieceType::Knight, Color::White));
        board[7][7] = Some(Piece::new(PieceType::Rook, Color::White));
        
        let mut state = Self {
            board,
            current_turn: Color::White,
            white_can_castle_kingside: true,
            white_can_castle_queenside: true,
            black_can_castle_kingside: true,
            black_can_castle_queenside: true,
            en_passant_target: None,
            halfmove_clock: 0,
            fullmove_number: 1,
            promotion_pending: None,
            position_history: HashMap::new(),
            current_hash: 0, // Will be calculated below
            draw_offered: None,
            white_timer: Timer::new(10, 0), // 10 minutes, no increment by default
            black_timer: Timer::new(10, 0),
            timer_enabled: false,
            move_cache: HashMap::new(),
        };
        
        state.current_hash = state.calculate_zobrist_hash();
        
        state.position_history.insert(state.current_hash, 1);
        
        state
    }
    
    fn update_position_history(&mut self) {
        *self.position_history.entry(self.current_hash).or_insert(0) += 1;
    }
    
    pub fn make_move(&mut self, from: (usize, usize), to: (usize, usize)) -> bool {
        if self.promotion_pending.is_some() {
            return false;
        }
        
        self.draw_offered = None;
        
        let (from_rank, from_file) = from;
        let (to_rank, to_file) = to;
        
        if let Some(mut piece) = self.board[from_rank][from_file] {
            if piece.color != self.current_turn {
                return false;
            }
            
            if self.would_be_in_check_after_move(from, to) {
                return false;
            }
            
            self.clear_move_cache();
            
            let old_en_passant = self.en_passant_target;
            if let Some((_rank, file)) = old_en_passant {
                self.current_hash ^= ZOBRIST.en_passant_keys[file];
            }
            
            self.en_passant_target = None;
            
            if piece.piece_type == PieceType::Pawn && old_en_passant == Some(to) && 
               from_file != to_file && self.board[to_rank][to_file].is_none() {
                let captured_pawn_rank = from_rank;
                let captured_pawn_file = to_file;
                
                self.update_hash_for_move(&piece, from, to);
                
                let captured_color = if piece.color == Color::White { BLACK } else { WHITE };
                let captured_square = captured_pawn_rank * 8 + captured_pawn_file;
                self.current_hash ^= ZOBRIST.piece_keys[captured_color][0][captured_square]; // Remove captured pawn
                
                piece.has_moved = true;
                self.board[to_rank][to_file] = Some(piece);
                self.board[from_rank][from_file] = None;
                
                self.board[captured_pawn_rank][captured_pawn_file] = None;
                
                self.halfmove_clock = 0;
                
                self.switch_turn();
                
                self.update_position_history();
                
                return true;
            }
            
            self.halfmove_clock += 1;
            
            if piece.piece_type == PieceType::Pawn && !piece.has_moved && 
               ((from_rank as isize - to_rank as isize).abs() == 2) {
                let direction = if piece.color == Color::White { -1 } else { 1 };
                let en_passant_rank = (from_rank as isize + direction) as usize;
                
                self.en_passant_target = Some((en_passant_rank, from_file));
                
                self.current_hash ^= ZOBRIST.en_passant_keys[from_file];
            }
            
            if piece.piece_type == PieceType::Pawn {
                self.halfmove_clock = 0;
            }
            
            if piece.piece_type == PieceType::King {
                if from_file + 2 == to_file && from_rank == to_rank {
                    if !self.can_castle_kingside(piece.color) {
                        return false;
                    }
                    
                    let mid_square = (from_rank, from_file + 1);
                    if self.would_be_in_check_after_move(from, mid_square) {
                        return false;
                    }
                    
                    self.update_hash_for_move(&piece, from, to);
                    
                    piece.has_moved = true;
                    self.board[to_rank][to_file] = Some(piece);
                    self.board[from_rank][from_file] = None;
                    
                    let rook_file = 7; // h-file
                    let rook_to_file = 5; // f-file
                    
                    if let Some(mut rook) = self.board[from_rank][rook_file] {
                        if rook.piece_type == PieceType::Rook && rook.color == piece.color {
                            self.update_hash_for_move(&rook, (from_rank, rook_file), (from_rank, rook_to_file));
                            
                            rook.has_moved = true;
                            self.board[from_rank][rook_to_file] = Some(rook);
                            self.board[from_rank][rook_file] = None;
                        }
                    }
                    
                    self.update_castling_flags(piece.color);
                    
                    self.switch_turn();
                    
                    self.update_position_history();
                    
                    return true;
                }
                
                if from_file as isize - 2 == to_file as isize && from_rank == to_rank {
                    if !self.can_castle_queenside(piece.color) {
                        return false;
                    }
                    
                    let mid_square = (from_rank, from_file - 1);
                    if self.would_be_in_check_after_move(from, mid_square) {
                        return false;
                    }
                    
                    self.update_hash_for_move(&piece, from, to);
                    
                    piece.has_moved = true;
                    self.board[to_rank][to_file] = Some(piece);
                    self.board[from_rank][from_file] = None;
                    
                    let rook_file = 0; // a-file
                    let rook_to_file = 3; // d-file
                    
                    if let Some(mut rook) = self.board[from_rank][rook_file] {
                        if rook.piece_type == PieceType::Rook && rook.color == piece.color {
                            self.update_hash_for_move(&rook, (from_rank, rook_file), (from_rank, rook_to_file));
                            
                            rook.has_moved = true;
                            self.board[from_rank][rook_to_file] = Some(rook);
                            self.board[from_rank][rook_file] = None;
                        }
                    }
                    
                    self.update_castling_flags(piece.color);
                    
                    self.switch_turn();
                    
                    self.update_position_history();
                    
                    return true;
                }
            }
            
            if piece.piece_type == PieceType::King {
                self.update_castling_flags(piece.color);
            } else if piece.piece_type == PieceType::Rook {
                if from_rank == 7 && from_file == 0 && piece.color == Color::White && self.white_can_castle_queenside {
                    self.current_hash ^= ZOBRIST.castling_keys[1]; // Toggle white queenside castling
                    self.white_can_castle_queenside = false;
                } else if from_rank == 7 && from_file == 7 && piece.color == Color::White && self.white_can_castle_kingside {
                    self.current_hash ^= ZOBRIST.castling_keys[0]; // Toggle white kingside castling
                    self.white_can_castle_kingside = false;
                } else if from_rank == 0 && from_file == 0 && piece.color == Color::Black && self.black_can_castle_queenside {
                    self.current_hash ^= ZOBRIST.castling_keys[3]; // Toggle black queenside castling
                    self.black_can_castle_queenside = false;
                } else if from_rank == 0 && from_file == 7 && piece.color == Color::Black && self.black_can_castle_kingside {
                    self.current_hash ^= ZOBRIST.castling_keys[2]; // Toggle black kingside castling
                    self.black_can_castle_kingside = false;
                }
            }
            
            let is_capture = self.board[to_rank][to_file].is_some();
            if is_capture {
                self.halfmove_clock = 0;
            }
            
            self.update_hash_for_move(&piece, from, to);
            
            piece.has_moved = true;
            
            self.board[to_rank][to_file] = Some(piece);
            self.board[from_rank][from_file] = None;
            
            if piece.piece_type == PieceType::Pawn {
                let promotion_rank = match piece.color {
                    Color::White => 0, // White pawns promote on the 8th rank (index 0)
                    Color::Black => 7, // Black pawns promote on the 1st rank (index 7)
                };
                
                if to_rank == promotion_rank {
                    self.promotion_pending = Some(PromotionState {
                        position: (to_rank, to_file),
                        color: piece.color,
                    });
                    
                    return true;
                }
            }
            
            self.switch_turn();
            
            self.update_position_history();
            
            return true;
        }
        
        false
    }
    
    pub fn promote_pawn(&mut self, piece_type: PieceType) -> bool {
        if let Some(promotion) = self.promotion_pending.take() {
            let (rank, file) = promotion.position;
            let color = promotion.color;
            let square = rank * 8 + file;
            
            let color_index = match color {
                Color::White => WHITE,
                Color::Black => BLACK,
            };
            self.current_hash ^= ZOBRIST.piece_keys[color_index][0][square]; // Remove pawn
            
            let piece_index = match piece_type {
                PieceType::Pawn => 0,
                PieceType::Knight => 1,
                PieceType::Bishop => 2,
                PieceType::Rook => 3,
                PieceType::Queen => 4,
                PieceType::King => 5,
            };
            self.current_hash ^= ZOBRIST.piece_keys[color_index][piece_index][square]; // Add new piece
            
            self.board[rank][file] = Some(Piece::new(piece_type, color));
            
            self.switch_turn();
            
            self.update_position_history();
            
            true
        } else {
            false
        }
    }
    
    fn switch_turn(&mut self) {
        if self.timer_enabled {
            // Stop the current player's timer and add increment
            match self.current_turn {
                Color::White => {
                    self.white_timer.stop();
                },
                Color::Black => {
                    self.black_timer.stop();
                }
            }
        }
        
        self.current_hash ^= ZOBRIST.side_to_move_key;
        
        self.current_turn = match self.current_turn {
            Color::White => Color::Black,
            Color::Black => {
                self.fullmove_number += 1;
                Color::White
            },
        };
        
        if self.timer_enabled {
            // Start the next player's timer
            match self.current_turn {
                Color::White => {
                    self.white_timer.start();
                },
                Color::Black => {
                    self.black_timer.start();
                }
            }
        }
    }
    
    fn update_castling_flags(&mut self, color: Color) {
        match color {
            Color::White => {
                if self.white_can_castle_kingside {
                    self.current_hash ^= ZOBRIST.castling_keys[0];
                    self.white_can_castle_kingside = false;
                }
                if self.white_can_castle_queenside {
                    self.current_hash ^= ZOBRIST.castling_keys[1];
                    self.white_can_castle_queenside = false;
                }
            },
            Color::Black => {
                if self.black_can_castle_kingside {
                    self.current_hash ^= ZOBRIST.castling_keys[2];
                    self.black_can_castle_kingside = false;
                }
                if self.black_can_castle_queenside {
                    self.current_hash ^= ZOBRIST.castling_keys[3];
                    self.black_can_castle_queenside = false;
                }
            },
        }
    }
    
    fn can_castle_kingside(&self, color: Color) -> bool {
        let can_castle = match color {
            Color::White => self.white_can_castle_kingside,
            Color::Black => self.black_can_castle_kingside,
        };
        
        if !can_castle {
            return false;
        }
        
        if self.is_in_check(color) {
            return false;
        }
        
        let rank = match color {
            Color::White => 7,
            Color::Black => 0,
        };
        
        if self.board[rank][4].is_none() ||
           self.board[rank][4].unwrap().piece_type != PieceType::King ||
           self.board[rank][4].unwrap().color != color ||
           self.board[rank][4].unwrap().has_moved {
            return false;
        }
        
        if self.board[rank][7].is_none() ||
           self.board[rank][7].unwrap().piece_type != PieceType::Rook ||
           self.board[rank][7].unwrap().color != color ||
           self.board[rank][7].unwrap().has_moved {
            return false;
        }
        
        if self.board[rank][5].is_some() || self.board[rank][6].is_some() {
            return false;
        }
        
        true
    }
    
    fn can_castle_queenside(&self, color: Color) -> bool {
        let can_castle = match color {
            Color::White => self.white_can_castle_queenside,
            Color::Black => self.black_can_castle_queenside,
        };
        
        if !can_castle {
            return false;
        }
        
        if self.is_in_check(color) {
            return false;
        }
        
        let rank = match color {
            Color::White => 7,
            Color::Black => 0,
        };
        
        if self.board[rank][4].is_none() ||
           self.board[rank][4].unwrap().piece_type != PieceType::King ||
           self.board[rank][4].unwrap().color != color ||
           self.board[rank][4].unwrap().has_moved {
            return false;
        }
        
        if self.board[rank][0].is_none() ||
           self.board[rank][0].unwrap().piece_type != PieceType::Rook ||
           self.board[rank][0].unwrap().color != color ||
           self.board[rank][0].unwrap().has_moved {
            return false;
        }
        
        if self.board[rank][1].is_some() || self.board[rank][2].is_some() || self.board[rank][3].is_some() {
            return false;
        }
        
        true
    }
    
    pub fn is_in_check(&self, color: Color) -> bool {
        let mut king_pos = None;
        
        for rank in 0..BOARD_SIZE {
            for file in 0..BOARD_SIZE {
                if let Some(piece) = self.board[rank][file] {
                    if piece.piece_type == PieceType::King && piece.color == color {
                        king_pos = Some((rank, file));
                        break;
                    }
                }
            }
            if king_pos.is_some() {
                break;
            }
        }
        
        if king_pos.is_none() {
            return false;
        }
        
        let (king_rank, king_file) = king_pos.unwrap();
        
        for rank in 0..BOARD_SIZE {
            for file in 0..BOARD_SIZE {
                if let Some(piece) = self.board[rank][file] {
                    if piece.color != color {
                        let moves = piece.get_possible_moves((rank, file), &self.board);
                        
                        if moves.contains(&(king_rank, king_file)) {
                            return true;
                        }
                    }
                }
            }
        }
        
        false
    }
    
    pub fn would_be_in_check_after_move(&self, from: (usize, usize), to: (usize, usize)) -> bool {
        let mut temp_board = self.clone();
        
        let (from_rank, from_file) = from;
        let (to_rank, to_file) = to;
        
        let piece_color = match temp_board.board[from_rank][from_file] {
            Some(piece) => piece.color,
            None => return false, // No piece to move
        };
        
        temp_board.board[to_rank][to_file] = temp_board.board[from_rank][from_file];
        temp_board.board[from_rank][from_file] = None;
        
        temp_board.is_in_check(piece_color)
    }
    
    pub fn is_checkmate(&self) -> bool {
        let color = self.current_turn;
        
        if !self.is_in_check(color) {
            return false;
        }
        
        !self.has_legal_moves()
    }
    
    pub fn is_stalemate(&self) -> bool {
        let color = self.current_turn;
        
        if self.is_in_check(color) {
            return false;
        }
        
        !self.has_legal_moves()
    }
    
    fn has_legal_moves(&self) -> bool {
        let mut clone = self.clone();
        clone.move_cache = self.move_cache.clone();
        
        let moves = clone.get_all_legal_moves();
        
        !moves.is_empty()
    }
    
    pub fn is_threefold_repetition(&self) -> bool {
        if let Some(count) = self.position_history.get(&self.current_hash) {
            return *count >= 3;
        }
        
        false
    }
    
    pub fn is_fifty_move_rule(&self) -> bool {
        self.halfmove_clock >= 100 // 50 moves from each player = 100 halfmoves
    }
    
    pub fn is_insufficient_material(&self) -> bool {
        let mut piece_counts = HashMap::new();
        
        for rank in 0..BOARD_SIZE {
            for file in 0..BOARD_SIZE {
                if let Some(piece) = self.board[rank][file] {
                    let key = (piece.piece_type, piece.color);
                    *piece_counts.entry(key).or_insert(0) += 1;
                }
            }
        }
        
        if piece_counts.len() == 2 {
            return true;
        }
        
        if piece_counts.len() == 3 {
            let has_only_bishop = piece_counts.iter().any(|(&(piece_type, _), &count)| 
                piece_type == PieceType::Bishop && count == 1);
                
            let has_only_knight = piece_counts.iter().any(|(&(piece_type, _), &count)| 
                piece_type == PieceType::Knight && count == 1);
                
            return has_only_bishop || has_only_knight;
        }
        
        if piece_counts.len() == 4 {
            let white_has_bishop = piece_counts.get(&(PieceType::Bishop, Color::White)).unwrap_or(&0) == &1;
            let black_has_bishop = piece_counts.get(&(PieceType::Bishop, Color::Black)).unwrap_or(&0) == &1;
            
            if white_has_bishop && black_has_bishop {
                let mut white_bishop_square = None;
                let mut black_bishop_square = None;
                
                for rank in 0..BOARD_SIZE {
                    for file in 0..BOARD_SIZE {
                        if let Some(piece) = self.board[rank][file] {
                            if piece.piece_type == PieceType::Bishop {
                                if piece.color == Color::White {
                                    white_bishop_square = Some((rank, file));
                                } else {
                                    black_bishop_square = Some((rank, file));
                                }
                            }
                        }
                    }
                }
                
                if let (Some((w_rank, w_file)), Some((b_rank, b_file))) = (white_bishop_square, black_bishop_square) {
                    return (w_rank + w_file) % 2 == (b_rank + b_file) % 2;
                }
            }
        }
        
        false
    }
    
    pub fn offer_draw(&mut self, color: Color) {
        self.draw_offered = Some(color);
    }
    
    pub fn accept_draw(&self) -> bool {
        match self.draw_offered {
            Some(color) => color != self.current_turn, // Can accept if the other player offered
            None => false,                             // No offer to accept
        }
    }
    
    pub fn is_draw(&self) -> bool {
        self.is_stalemate() || 
        self.is_threefold_repetition() || 
        self.is_fifty_move_rule() || 
        self.is_insufficient_material()
    }
    
    fn clone(&self) -> Self {
        let mut new_board = [[None; BOARD_SIZE]; BOARD_SIZE];
        
        for rank in 0..BOARD_SIZE {
            for file in 0..BOARD_SIZE {
                new_board[rank][file] = self.board[rank][file];
            }
        }
        
        Self {
            board: new_board,
            current_turn: self.current_turn,
            white_can_castle_kingside: self.white_can_castle_kingside,
            white_can_castle_queenside: self.white_can_castle_queenside,
            black_can_castle_kingside: self.black_can_castle_kingside,
            black_can_castle_queenside: self.black_can_castle_queenside,
            en_passant_target: self.en_passant_target,
            halfmove_clock: self.halfmove_clock,
            fullmove_number: self.fullmove_number,
            promotion_pending: None, // Don't need to copy this for simulation
            position_history: HashMap::new(), // Don't need to copy history for simulation
            current_hash: self.current_hash, // Copy the hash
            draw_offered: None, // Don't need to copy for simulation
            white_timer: self.white_timer,
            black_timer: self.black_timer,
            timer_enabled: self.timer_enabled,
            move_cache: HashMap::new(), // Don't need to copy move cache for simulation
        }
    }
    
    fn calculate_zobrist_hash(&self) -> u64 {
        use crate::zobrist::ZOBRIST;
        
        let mut hash = 0u64;
        
        for rank in 0..BOARD_SIZE {
            for file in 0..BOARD_SIZE {
                if let Some(piece) = self.board[rank][file] {
                    let square = rank * 8 + file;
                    let color_index = match piece.color {
                        Color::White => WHITE,
                        Color::Black => BLACK,
                    };
                    let piece_index = match piece.piece_type {
                        PieceType::Pawn => 0,
                        PieceType::Knight => 1,
                        PieceType::Bishop => 2,
                        PieceType::Rook => 3,
                        PieceType::Queen => 4,
                        PieceType::King => 5,
                    };
                    
                    hash ^= ZOBRIST.piece_keys[color_index][piece_index][square];
                }
            }
        }
        
        if self.white_can_castle_kingside {
            hash ^= ZOBRIST.castling_keys[0];
        }
        if self.white_can_castle_queenside {
            hash ^= ZOBRIST.castling_keys[1];
        }
        if self.black_can_castle_kingside {
            hash ^= ZOBRIST.castling_keys[2];
        }
        if self.black_can_castle_queenside {
            hash ^= ZOBRIST.castling_keys[3];
        }
        
        if let Some((_rank, file)) = self.en_passant_target {
            hash ^= ZOBRIST.en_passant_keys[file];
        }
        
        if self.current_turn == Color::Black {
            hash ^= ZOBRIST.side_to_move_key;
        }
        
        hash
    }
    
    fn update_hash_for_move(&mut self, piece: &Piece, from: (usize, usize), to: (usize, usize)) {
        let (from_rank, from_file) = from;
        let (to_rank, to_file) = to;
        let from_square = from_rank * 8 + from_file;
        let to_square = to_rank * 8 + to_file;
        
        let color_index = match piece.color {
            Color::White => WHITE,
            Color::Black => BLACK,
        };
        let piece_index = match piece.piece_type {
            PieceType::Pawn => 0,
            PieceType::Knight => 1,
            PieceType::Bishop => 2,
            PieceType::Rook => 3,
            PieceType::Queen => 4,
            PieceType::King => 5,
        };
        
        self.current_hash ^= ZOBRIST.piece_keys[color_index][piece_index][from_square];
        
        if let Some(captured) = self.board[to_rank][to_file] {
            let cap_color_index = match captured.color {
                Color::White => WHITE,
                Color::Black => BLACK,
            };
            let cap_piece_index = match captured.piece_type {
                PieceType::Pawn => 0,
                PieceType::Knight => 1,
                PieceType::Bishop => 2,
                PieceType::Rook => 3,
                PieceType::Queen => 4,
                PieceType::King => 5,
            };
            self.current_hash ^= ZOBRIST.piece_keys[cap_color_index][cap_piece_index][to_square];
        }
        
        self.current_hash ^= ZOBRIST.piece_keys[color_index][piece_index][to_square];
    }
    
    pub fn get_all_legal_moves(&mut self) -> Vec<((usize, usize), (usize, usize))> {
        if let Some(moves) = self.move_cache.get(&self.current_hash) {
            return moves.clone();
        }
        
        let current_color = self.current_turn;
        let mut legal_moves = Vec::new();
        
        for from_rank in 0..BOARD_SIZE {
            for from_file in 0..BOARD_SIZE {
                if let Some(piece) = self.board[from_rank][from_file] {
                    if piece.color == current_color {
                        let moves = piece.get_possible_moves((from_rank, from_file), &self.board);
                        
                        for to_pos in moves {
                            if !self.would_be_in_check_after_move((from_rank, from_file), to_pos) {
                                legal_moves.push(((from_rank, from_file), to_pos));
                            }
                        }
                    }
                }
            }
        }
        
        self.move_cache.insert(self.current_hash, legal_moves.clone());
        
        legal_moves
    }
    
    fn clear_move_cache(&mut self) {
        self.move_cache.clear();
    }
    
    pub fn set_timers(&mut self, minutes: u64, increment_seconds: u64) {
        self.white_timer = Timer::new(minutes, increment_seconds);
        self.black_timer = Timer::new(minutes, increment_seconds);
        self.timer_enabled = true;
    }
    
    pub fn disable_timers(&mut self) {
        println!("Disabling timers in GameState");
        
        // Stop any running timers first
        if self.timer_enabled {
            match self.current_turn {
                Color::White => { self.white_timer.stop(); },
                Color::Black => { self.black_timer.stop(); }
            }
        }
        
        self.timer_enabled = false;
        
        // Reset last_update to avoid counting elapsed time while disabled
        self.white_timer.last_update = None;
        self.black_timer.last_update = None;
        self.white_timer.is_running = false;
        self.black_timer.is_running = false;
    }
    
    pub fn enable_timers(&mut self) {
        println!("Enabling timers in GameState for {:?}", self.current_turn);
        self.timer_enabled = true;
        
        // Ensure previous timers are stopped
        self.white_timer.is_running = false;
        self.black_timer.is_running = false;
        
        // Start only the current player's timer
        match self.current_turn {
            Color::White => {
                self.white_timer.start();
                println!("White timer is now running: {}", self.white_timer.is_running);
            },
            Color::Black => {
                self.black_timer.start();
                println!("Black timer is now running: {}", self.black_timer.is_running);
            }
        }
    }
    
    pub fn reset_timers(&mut self) {
        self.white_timer.reset();
        self.black_timer.reset();
    }
    
    pub fn get_current_timer(&self) -> &Timer {
        match self.current_turn {
            Color::White => &self.white_timer,
            Color::Black => &self.black_timer,
        }
    }
    
    pub fn get_current_timer_mut(&mut self) -> &mut Timer {
        match self.current_turn {
            Color::White => &mut self.white_timer,
            Color::Black => &mut self.black_timer,
        }
    }
    
    pub fn update_timers(&mut self) -> bool {
        if !self.timer_enabled {
            return false;
        }
        
        let result = match self.current_turn {
            Color::White => {
                let time_before = self.white_timer.remaining.as_secs();
                let flag_fallen = self.white_timer.update();
                let time_after = self.white_timer.remaining.as_secs();
                
                if time_before != time_after {
                    println!("White timer updated: {} -> {}", 
                             time_before, time_after);
                }
                
                flag_fallen
            },
            Color::Black => {
                let time_before = self.black_timer.remaining.as_secs();
                let flag_fallen = self.black_timer.update();
                let time_after = self.black_timer.remaining.as_secs();
                
                if time_before != time_after {
                    println!("Black timer updated: {} -> {}", 
                             time_before, time_after);
                }
                
                flag_fallen
            },
        };
        
        result
    }
    
    pub fn is_time_up(&self) -> bool {
        if !self.timer_enabled {
            return false;
        }
        
        self.white_timer.remaining.as_secs() == 0 || self.black_timer.remaining.as_secs() == 0
    }
} 
