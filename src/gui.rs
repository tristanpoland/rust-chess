use ggez::{Context, GameResult};
use ggez::graphics::{self, Canvas, Color as GgezColor, DrawParam, Rect, Text};
use ggez::input::mouse::MouseButton;
use ggez::mint::{Point2, Vector2};

use crate::board::{GameState, BOARD_SIZE, PromotionState};
use crate::piece::{PieceType, Color, Piece};
use crate::assets::Assets;

const SQUARE_SIZE: f32 = 60.0;
const BOARD_OFFSET_X: f32 = 50.0;
const BOARD_OFFSET_Y: f32 = 50.0;

const LIGHT_SQUARE: GgezColor = GgezColor::new(0.9, 0.9, 0.8, 1.0);
const DARK_SQUARE: GgezColor = GgezColor::new(0.5, 0.5, 0.4, 1.0);
const SELECTED_SQUARE: GgezColor = GgezColor::new(0.7, 0.9, 0.7, 1.0);
const POSSIBLE_MOVE: GgezColor = GgezColor::new(0.7, 0.7, 0.9, 0.7);
const PROMOTION_BG: GgezColor = GgezColor::new(0.3, 0.3, 0.3, 0.9);
const BUTTON_BG: GgezColor = GgezColor::new(0.3, 0.3, 0.6, 1.0);
const BUTTON_HOVER: GgezColor = GgezColor::new(0.4, 0.4, 0.7, 1.0);

const BUTTON_WIDTH: f32 = 120.0;
const BUTTON_HEIGHT: f32 = 30.0;
const BUTTON_MARGIN: f32 = 20.0;

pub struct Button {
    rect: Rect,
    text: String,
    hovered: bool,
}

impl Button {
    fn new(x: f32, y: f32, width: f32, height: f32, text: &str) -> Self {
        Self {
            rect: Rect::new(x, y, width, height),
            text: text.to_string(),
            hovered: false,
        }
    }
    
    fn contains(&self, point: Point2<f32>) -> bool {
        self.rect.contains(point)
    }
    
    fn set_hover(&mut self, hovered: bool) {
        self.hovered = hovered;
    }
    
    fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult<()> {
        let color = if self.hovered { BUTTON_HOVER } else { BUTTON_BG };
        
        let mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            self.rect,
            color,
        )?;
        canvas.draw(&mesh, DrawParam::default());
        
        let mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::stroke(2.0),
            self.rect,
            GgezColor::WHITE,
        )?;
        canvas.draw(&mesh, DrawParam::default());
        
        let text = Text::new(&self.text);
        
        let dest = Point2 {
            x: self.rect.x + (self.rect.w / 2.0),
            y: self.rect.y + (self.rect.h / 2.0),
        };
        
        canvas.draw(
            &text,
            DrawParam::default()
                .dest(dest)
                .offset(Point2 { x: 0.5, y: 0.5 }) // Center the text
                .color(GgezColor::WHITE)
        );
        
        Ok(())
    }
}

pub struct MoveInfo {
    pub from: (u8, u8),
    pub to: (u8, u8),
    pub promotion: Option<char>,
}

pub struct ChessGui {
    game_state: GameState,
    selected_square: Option<(usize, usize)>,
    possible_moves: Vec<(usize, usize)>,
    assets: Assets,
    show_square_coordinates: bool,
    game_over: bool,
    needs_redraw: bool,
    is_network_game: bool,
    player_color: Option<Color>,
}

impl ChessGui {
    pub fn new(ctx: &mut Context) -> GameResult<Self> {
        let game_state = GameState::new();
        let assets = Assets::new(ctx)?;
        
        Ok(Self {
            game_state,
            selected_square: None,
            possible_moves: Vec::new(),
            assets,
            show_square_coordinates: true,
            game_over: false,
            needs_redraw: true,
            is_network_game: false,
            player_color: None,
        })
    }
    
    pub fn set_player_color(&mut self, is_white: bool) {
        self.player_color = Some(if is_white { Color::White } else { Color::Black });
        self.is_network_game = true;
        self.needs_redraw = true;
    }
    
    pub fn handle_network_move(&mut self, from: (u8, u8), to: (u8, u8), promotion: Option<char>) -> GameResult<()> {
        let from = (from.0 as usize, from.1 as usize);
        let to = (to.0 as usize, to.1 as usize);
        
        if let Some(promotion) = promotion {
            let piece_type = match promotion {
                'Q' => PieceType::Queen,
                'R' => PieceType::Rook,
                'B' => PieceType::Bishop,
                'N' => PieceType::Knight,
                _ => return Ok(()),
            };
            if !self.game_state.promote_pawn(piece_type) {
                return Ok(());
            }
        }
        
        if !self.game_state.make_move(from, to) {
            return Ok(());
        }
        
        self.selected_square = None;
        self.possible_moves.clear();
        self.needs_redraw = true;
        
        Ok(())
    }
    
    pub fn update_game_state(&mut self, board: [[Option<(PieceType, Color)>; 8]; 8], current_turn: Color, promotion_pending: Option<(usize, usize, Color)>, game_over: bool) -> GameResult<()> {
        // Update the board
        for rank in 0..8 {
            for file in 0..8 {
                self.game_state.board[rank][file] = board[rank][file].map(|(piece_type, color)| {
                    Piece::new(piece_type, color)
                });
            }
        }

        // Update other game state
        self.game_state.current_turn = current_turn;
        self.game_state.promotion_pending = promotion_pending.map(|(rank, file, color)| 
            PromotionState {
                position: (rank, file),
                color,
            }
        );
        self.game_over = game_over;

        // Clear selection and possible moves
        self.selected_square = None;
        self.possible_moves.clear();
        self.needs_redraw = true;

        Ok(())
    }
    
    pub fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        if !self.needs_redraw {
            return Ok(());
        }
        
        let mut canvas = Canvas::from_frame(ctx, GgezColor::new(0.2, 0.2, 0.2, 1.0));
        
        self.draw_board(ctx, &mut canvas)?;
        
        self.draw_pieces(&mut canvas);
        
        self.draw_status(&mut canvas)?;
        
        if self.game_state.promotion_pending.is_some() {
            self.draw_promotion_dialog(ctx, &mut canvas)?;
        }
        
        canvas.finish(ctx)?;
        
        self.needs_redraw = false;
        
        Ok(())
    }
    
    fn draw_board(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult<()> {
        for rank in 0..BOARD_SIZE {
            for file in 0..BOARD_SIZE {
                // Invert coordinates if playing as black
                let (display_rank, display_file) = self.get_display_coordinates(rank, file);
                
                let x = BOARD_OFFSET_X + (display_file as f32) * SQUARE_SIZE;
                let y = BOARD_OFFSET_Y + (display_rank as f32) * SQUARE_SIZE;
                
                let is_light = (rank + file) % 2 == 0;
                let color = if is_light { LIGHT_SQUARE } else { DARK_SQUARE };
                
                let color = if Some((rank, file)) == self.selected_square {
                    SELECTED_SQUARE
                } else if self.possible_moves.contains(&(rank, file)) {
                    POSSIBLE_MOVE
                } else {
                    color
                };
                
                let square = Rect::new(x, y, SQUARE_SIZE, SQUARE_SIZE);
                let mesh = graphics::Mesh::new_rectangle(
                    ctx,
                    graphics::DrawMode::fill(),
                    square,
                    color,
                )?;
                canvas.draw(&mesh, DrawParam::default());
                
                let mesh = graphics::Mesh::new_rectangle(
                    ctx,
                    graphics::DrawMode::stroke(1.0),
                    square,
                    GgezColor::new(0.0, 0.0, 0.0, 0.3),
                )?;
                canvas.draw(&mesh, DrawParam::default());
                
                // Draw square coordinates when enabled
                if self.show_square_coordinates {
                    let file_char = (b'a' + file as u8) as char;
                    let rank_num = 8 - rank;
                    let coord_text = Text::new(format!("{}{}", file_char, rank_num));
                    
                    // Position in the top-left corner of each square
                    let coord_x = x + 5.0;
                    let coord_y = y + 5.0;
                    
                    // Use contrasting color for better visibility
                    let text_color = if is_light { 
                        GgezColor::new(0.2, 0.2, 0.2, 0.8) 
                    } else { 
                        GgezColor::new(0.9, 0.9, 0.9, 0.8) 
                    };
                    
                    canvas.draw(
                        &coord_text,
                        DrawParam::default()
                            .dest(Point2 { x: coord_x, y: coord_y })
                            .color(text_color)
                            .scale(Vector2 { x: 0.8, y: 0.8 })
                    );
                }
            }
        }
        
        Ok(())
    }
    
    fn draw_pieces(&self, canvas: &mut Canvas) {
        for rank in 0..BOARD_SIZE {
            for file in 0..BOARD_SIZE {
                if let Some(piece) = self.game_state.board[rank][file] {
                    // Invert coordinates if playing as black
                    let (display_rank, display_file) = self.get_display_coordinates(rank, file);
                    
                    let x = BOARD_OFFSET_X + (display_file as f32) * SQUARE_SIZE;
                    let y = BOARD_OFFSET_Y + (display_rank as f32) * SQUARE_SIZE;
                    
                    let dest = Point2 { 
                        x: x + SQUARE_SIZE / 2.0, 
                        y: y + SQUARE_SIZE / 2.0 
                    };
                    
                    self.assets.draw_piece(
                        canvas,
                        piece.piece_type,
                        piece.color,
                        DrawParam::default()
                            .dest(dest)
                            .offset(Point2 { x: 0.5, y: 0.5 }) // Center the image
                            .scale(Vector2 { x: 0.45, y: 0.45 }) // Scale to fit the square
                    );
                }
            }
        }
    }
    
    fn draw_status(&self, canvas: &mut Canvas) -> GameResult<()> {
        let mut status_text = format!("Current turn: {:?}", self.game_state.current_turn);
        
        if self.game_state.is_in_check(self.game_state.current_turn) {
            if self.game_state.is_checkmate() {
                status_text = format!("{:?} is in CHECKMATE!", self.game_state.current_turn);
            } else {
                status_text = format!("{:?} is in CHECK!", self.game_state.current_turn);
            }
        } else if self.game_state.is_stalemate() {
            status_text = "STALEMATE!".to_string();
        } else if self.game_state.is_threefold_repetition() {
            status_text = "DRAW by threefold repetition!".to_string();
        } else if self.game_state.is_fifty_move_rule() {
            status_text = "DRAW by fifty-move rule!".to_string();
        } else if self.game_state.is_insufficient_material() {
            status_text = "DRAW by insufficient material!".to_string();
        }
        
        let status_display = Text::new(status_text);
        
        // Position status text at the left side below the board
        canvas.draw(
            &status_display,
            DrawParam::default()
                .dest(Point2 {
                    x: BOARD_OFFSET_X,
                    y: BOARD_OFFSET_Y + (BOARD_SIZE as f32) * SQUARE_SIZE + 40.0,
                })
                .color(GgezColor::WHITE)
        );
        
        let halfmove_text = Text::new(format!("Halfmove clock: {}", self.game_state.halfmove_clock));
        
        // Position halfmove clock under status text
        canvas.draw(
            &halfmove_text,
            DrawParam::default()
                .dest(Point2 {
                    x: BOARD_OFFSET_X,
                    y: BOARD_OFFSET_Y + (BOARD_SIZE as f32) * SQUARE_SIZE + 60.0,
                })
                .color(GgezColor::WHITE)
        );
        
        Ok(())
    }
    
    fn draw_promotion_dialog(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult<()> {
        if let Some(ref promotion) = self.game_state.promotion_pending {
            let (rank, file) = promotion.position;
            let color = promotion.color;
            
            // Invert coordinates if playing as black
            let (display_rank, display_file) = self.get_display_coordinates(rank, file);
            
            let square_x = BOARD_OFFSET_X + (display_file as f32) * SQUARE_SIZE;
            let square_y = BOARD_OFFSET_Y + (display_rank as f32) * SQUARE_SIZE;
            
            let dialog_width = SQUARE_SIZE;
            let dialog_height = SQUARE_SIZE * 4.0; // Space for 4 pieces
            
            // Adjust dialog position based on perspective
            let dialog_y = if self.is_inverted_board() {
                if rank > 3 {
                    square_y // Dialog extends downward
                } else {
                    square_y - dialog_height + SQUARE_SIZE // Dialog extends upward
                }
            } else {
                if rank < 4 {
                    square_y // Dialog extends downward
                } else {
                    square_y - dialog_height + SQUARE_SIZE // Dialog extends upward
                }
            };
            
            let dialog_rect = Rect::new(square_x, dialog_y, dialog_width, dialog_height);
            let dialog_mesh = graphics::Mesh::new_rectangle(
                ctx,
                graphics::DrawMode::fill(),
                dialog_rect,
                PROMOTION_BG,
            )?;
            canvas.draw(&dialog_mesh, DrawParam::default());
            
            let promotion_pieces = [PieceType::Queen, PieceType::Rook, PieceType::Bishop, PieceType::Knight];
            
            for (i, &piece_type) in promotion_pieces.iter().enumerate() {
                let piece_y = dialog_y + (i as f32) * SQUARE_SIZE;
                let dest = Point2 { 
                    x: square_x + SQUARE_SIZE / 2.0,
                    y: piece_y + SQUARE_SIZE / 2.0,
                };
                
                self.assets.draw_piece(
                    canvas,
                    piece_type,
                    color,
                    DrawParam::default()
                        .dest(dest)
                        .offset(Point2 { x: 0.5, y: 0.5 })
                        .scale(Vector2 { x: 0.45, y: 0.45 })
                );
            }
        }
        
        Ok(())
    }
    
    pub fn handle_mouse_down(&mut self, button: MouseButton, x: f32, y: f32) -> GameResult<Option<MoveInfo>> {
        if button != MouseButton::Left {
            return Ok(None);
        }

        if self.game_over {
            return Ok(None);
        }

        if self.game_state.promotion_pending.is_some() {
            self.handle_promotion_selection(x, y)?;
            return Ok(None);
        }

        if self.is_network_game {
            if let Some(player_color) = self.player_color {
                if player_color != self.game_state.current_turn {
                    return Ok(None);
                }
            }
        }

        let (rank, file) = self.get_square_from_coords(x, y);
        
        if let Some(selected) = self.selected_square {
            if self.possible_moves.contains(&(rank, file)) {
                let from = (selected.0 as u8, selected.1 as u8);
                let to = (rank as u8, file as u8);
                
                if self.game_state.make_move(selected, (rank, file)) {
                    self.selected_square = None;
                    self.possible_moves.clear();
                    self.needs_redraw = true;
                    
                    if self.game_state.promotion_pending.is_some() {
                        return Ok(Some(MoveInfo { from, to, promotion: None }));
                    }
                    
                    return Ok(Some(MoveInfo { from, to, promotion: None }));
                }
            }
            self.selected_square = None;
            self.possible_moves.clear();
        }

        if let Some(piece) = self.game_state.board[rank][file] {
            if self.is_network_game {
                if let Some(player_color) = self.player_color {
                    if piece.color != player_color {
                        return Ok(None);
                    }
                }
            }
            
            if piece.color == self.game_state.current_turn {
                self.selected_square = Some((rank, file));
                self.possible_moves = self.game_state.get_all_legal_moves()
                    .iter()
                    .filter(|&&(from, _)| from == (rank, file))
                    .map(|&(_, to)| to)
                    .collect();
            }
        }

        self.needs_redraw = true;
        Ok(None)
    }
    
    pub fn handle_mouse_move(&mut self, x: f32, y: f32) -> GameResult<()> {
        Ok(())
    }
    
    pub fn update(&mut self) -> GameResult<()> {
        // No timer updates needed anymore
        Ok(())
    }
    
    fn check_game_end(&mut self) {
        if self.game_state.is_checkmate() || 
           self.game_state.is_stalemate() || 
           self.game_state.is_draw() {
            self.game_over = true;
            self.needs_redraw = true;
        }
    }

    fn handle_promotion_selection(&mut self, x: f32, y: f32) -> GameResult<()> {
        if let Some(ref promotion) = self.game_state.promotion_pending {
            let (rank, file) = promotion.position;
            
            // Invert coordinates if playing as black
            let (display_rank, display_file) = self.get_display_coordinates(rank, file);
            
            let square_x = BOARD_OFFSET_X + (display_file as f32) * SQUARE_SIZE;
            let square_y = BOARD_OFFSET_Y + (display_rank as f32) * SQUARE_SIZE;
            
            let dialog_width = SQUARE_SIZE;
            let dialog_height = SQUARE_SIZE * 4.0;
            
            // Adjust dialog position based on perspective
            let dialog_y = if self.is_inverted_board() {
                if rank > 3 {
                    square_y // Dialog extends downward
                } else {
                    square_y - dialog_height + SQUARE_SIZE // Dialog extends upward
                }
            } else {
                if rank < 4 {
                    square_y // Dialog extends downward
                } else {
                    square_y - dialog_height + SQUARE_SIZE // Dialog extends upward
                }
            };
            
            if x >= square_x && x < square_x + dialog_width && 
               y >= dialog_y && y < dialog_y + dialog_height {
                
                let relative_y = y - dialog_y;
                let piece_index = (relative_y / SQUARE_SIZE) as usize;
                
                if piece_index < 4 {
                    let promotion_pieces = [PieceType::Queen, PieceType::Rook, PieceType::Bishop, PieceType::Knight];
                    let selected_piece = promotion_pieces[piece_index];
                    
                    self.game_state.promote_pawn(selected_piece);
                    
                    self.check_game_end();
                    
                    self.needs_redraw = true;
                }
            }
        }
        Ok(())
    }

    fn get_square_from_coords(&self, x: f32, y: f32) -> (usize, usize) {
        let display_file = ((x - BOARD_OFFSET_X) / SQUARE_SIZE) as usize;
        let display_rank = ((y - BOARD_OFFSET_Y) / SQUARE_SIZE) as usize;
        
        // Convert display coordinates to internal coordinates
        self.get_internal_coordinates(display_rank, display_file)
    }
    
    // Helper method to check if board should be inverted
    fn is_inverted_board(&self) -> bool {
        matches!(self.player_color, Some(Color::Black))
    }
    
    // Convert internal coordinates to display coordinates based on perspective
    fn get_display_coordinates(&self, rank: usize, file: usize) -> (usize, usize) {
        if self.is_inverted_board() {
            // For black perspective: flip both rank and file
            (7 - rank, 7 - file) 
        } else {
            // For white perspective: use coordinates as-is
            (rank, file)
        }
    }
    
    // Convert display coordinates to internal coordinates based on perspective
    fn get_internal_coordinates(&self, display_rank: usize, display_file: usize) -> (usize, usize) {
        if self.is_inverted_board() {
            // For black perspective: flip both rank and file
            (7 - display_rank, 7 - display_file)
        } else {
            // For white perspective: use coordinates as-is
            (display_rank, display_file)
        }
    }
} 
