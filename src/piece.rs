#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn opposite(&self) -> Self {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Piece {
    pub piece_type: PieceType,
    pub color: Color,
    pub has_moved: bool,
}

impl Piece {
    pub fn new(piece_type: PieceType, color: Color) -> Self {
        Self {
            piece_type,
            color,
            has_moved: false,
        }
    }
    
    pub fn get_possible_moves(&self, position: (usize, usize), board: &[[Option<Piece>; 8]; 8]) -> Vec<(usize, usize)> {
        let (rank, file) = position;
        let mut moves = Vec::new();
        
        match self.piece_type {
            PieceType::Pawn => {
                let direction = if self.color == Color::White { -1isize } else { 1isize };
                
                let new_rank = (rank as isize + direction) as usize;
                if new_rank < 8 && board[new_rank][file].is_none() {
                    moves.push((new_rank, file));
                    
                    if !self.has_moved {
                        let double_rank = (rank as isize + 2 * direction) as usize;
                        if double_rank < 8 && board[double_rank][file].is_none() {
                            moves.push((double_rank, file));
                        }
                    }
                }
                
                for file_offset in [-1, 1] {
                    let new_file = file as isize + file_offset;
                    if new_file >= 0 && new_file < 8 {
                        let new_rank = (rank as isize + direction) as usize;
                        if new_rank < 8 {
                            // Normal diagonal capture
                            if let Some(piece) = board[new_rank][new_file as usize] {
                                if piece.color != self.color {
                                    moves.push((new_rank, new_file as usize));
                                }
                            }
                            
                            // En passant - check if there's an opponent's pawn adjacent that just moved two squares
                            // The actual en passant logic is handled in the board.rs make_move function
                            // This just adds the potential en passant capture as a possible move
                            if board[new_rank][new_file as usize].is_none() && // Target square is empty
                               board[rank][new_file as usize].is_some() && // Adjacent square has a piece
                               board[rank][new_file as usize].unwrap().piece_type == PieceType::Pawn && // It's a pawn
                               board[rank][new_file as usize].unwrap().color != self.color { // It's an opponent's pawn
                                // The actual en passant validation will be done in make_move
                                moves.push((new_rank, new_file as usize));
                            }
                        }
                    }
                }
            },
            PieceType::Knight => {
                let knight_moves = [
                    (-2, -1), (-2, 1), (-1, -2), (-1, 2),
                    (1, -2), (1, 2), (2, -1), (2, 1),
                ];
                
                for (rank_offset, file_offset) in knight_moves {
                    let new_rank = rank as isize + rank_offset;
                    let new_file = file as isize + file_offset;
                    
                    if new_rank >= 0 && new_rank < 8 && new_file >= 0 && new_file < 8 {
                        let new_rank = new_rank as usize;
                        let new_file = new_file as usize;
                        
                        if let Some(piece) = board[new_rank][new_file] {
                            if piece.color != self.color {
                                moves.push((new_rank, new_file));
                            }
                        } else {
                            moves.push((new_rank, new_file));
                        }
                    }
                }
            },
            PieceType::Bishop => {
                self.add_diagonal_moves(rank, file, board, &mut moves);
            },
            PieceType::Rook => {
                self.add_straight_moves(rank, file, board, &mut moves);
            },
            PieceType::Queen => {
                self.add_diagonal_moves(rank, file, board, &mut moves);
                self.add_straight_moves(rank, file, board, &mut moves);
            },
            PieceType::King => {
                for rank_offset in -1..=1 {
                    for file_offset in -1..=1 {
                        if rank_offset == 0 && file_offset == 0 {
                            continue;
                        }
                        
                        let new_rank = rank as isize + rank_offset;
                        let new_file = file as isize + file_offset;
                        
                        if new_rank >= 0 && new_rank < 8 && new_file >= 0 && new_file < 8 {
                            let new_rank = new_rank as usize;
                            let new_file = new_file as usize;
                            
                            if let Some(piece) = board[new_rank][new_file] {
                                if piece.color != self.color {
                                    moves.push((new_rank, new_file));
                                }
                            } else {
                                moves.push((new_rank, new_file));
                            }
                        }
                    }
                }
                
                if !self.has_moved {
                    let king_rank = match self.color {
                        Color::White => 7,
                        Color::Black => 0,
                    };
                    
                    if rank == king_rank && file == 4 {
                        if board[king_rank][5].is_none() && board[king_rank][6].is_none() {
                            if let Some(rook) = board[king_rank][7] {
                                if rook.piece_type == PieceType::Rook && 
                                   rook.color == self.color && 
                                   !rook.has_moved {
                                    moves.push((king_rank, 6));
                                }
                            }
                        }
                        
                        if board[king_rank][1].is_none() && 
                           board[king_rank][2].is_none() && 
                           board[king_rank][3].is_none() {
                            if let Some(rook) = board[king_rank][0] {
                                if rook.piece_type == PieceType::Rook && 
                                   rook.color == self.color && 
                                   !rook.has_moved {
                                    moves.push((king_rank, 2));
                                }
                            }
                        }
                    }
                }
            },
        }
        
        moves
    }
    
    fn add_diagonal_moves(&self, rank: usize, file: usize, board: &[[Option<Piece>; 8]; 8], moves: &mut Vec<(usize, usize)>) {
        let directions = [(-1, -1), (-1, 1), (1, -1), (1, 1)];
        
        for (rank_dir, file_dir) in directions {
            let mut new_rank = rank as isize;
            let mut new_file = file as isize;
            
            loop {
                new_rank += rank_dir;
                new_file += file_dir;
                
                if new_rank < 0 || new_rank >= 8 || new_file < 0 || new_file >= 8 {
                    break;
                }
                
                let new_rank = new_rank as usize;
                let new_file = new_file as usize;
                
                if let Some(piece) = board[new_rank][new_file] {
                    if piece.color != self.color {
                        moves.push((new_rank, new_file));
                    }
                    break;
                } else {
                    moves.push((new_rank, new_file));
                }
            }
        }
    }
    
    fn add_straight_moves(&self, rank: usize, file: usize, board: &[[Option<Piece>; 8]; 8], moves: &mut Vec<(usize, usize)>) {
        let directions = [(-1, 0), (0, 1), (1, 0), (0, -1)];
        
        for (rank_dir, file_dir) in directions {
            let mut new_rank = rank as isize;
            let mut new_file = file as isize;
            
            loop {
                new_rank += rank_dir;
                new_file += file_dir;
                
                if new_rank < 0 || new_rank >= 8 || new_file < 0 || new_file >= 8 {
                    break;
                }
                
                let new_rank = new_rank as usize;
                let new_file = new_file as usize;
                
                if let Some(piece) = board[new_rank][new_file] {
                    if piece.color != self.color {
                        moves.push((new_rank, new_file));
                    }
                    break;
                } else {
                    moves.push((new_rank, new_file));
                }
            }
        }
    }
    
    pub fn to_char(&self) -> char {
        match (self.piece_type, self.color) {
            (PieceType::Pawn, Color::White) => '♙',
            (PieceType::Knight, Color::White) => '♘',
            (PieceType::Bishop, Color::White) => '♗',
            (PieceType::Rook, Color::White) => '♖',
            (PieceType::Queen, Color::White) => '♕',
            (PieceType::King, Color::White) => '♔',
            (PieceType::Pawn, Color::Black) => '♟',
            (PieceType::Knight, Color::Black) => '♞',
            (PieceType::Bishop, Color::Black) => '♝',
            (PieceType::Rook, Color::Black) => '♜',
            (PieceType::Queen, Color::Black) => '♛',
            (PieceType::King, Color::Black) => '♚',
        }
    }
} 
