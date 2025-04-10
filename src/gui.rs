use ggez::{Context, GameResult};
use ggez::graphics::{self, Canvas, Color as GgezColor, DrawParam, Rect, Text};
use ggez::input::mouse::MouseButton;
use ggez::mint::{Point2, Vector2};
use std::time::{Instant, Duration};

use crate::board::{GameState, BOARD_SIZE};
use crate::piece::{PieceType, Color};
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

pub const TIMER_DISPLAY_HEIGHT: f32 = 25.0;
pub const TIMER_DISPLAY_WIDTH: f32 = 100.0;
pub const TIMER_Y_OFFSET: f32 = 20.0;

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

pub struct ChessGui {
    game_state: GameState,
    selected_square: Option<(usize, usize)>,
    possible_moves: Vec<(usize, usize)>,
    assets: Assets,
    offer_draw_button: Button,
    accept_draw_button: Button,
    restart_button: Button,
    timer_toggle_button: Button,
    timer_config_button: Button,
    show_coordinates_button: Button,
    show_square_coordinates: bool,
    game_over: bool,
    needs_redraw: bool, // Flag to control redrawing
    timer_options: Vec<(u64, u64)>, // [(minutes, increment), ...]
    selected_timer_option: usize,
    last_timer_update: Instant,
}

impl ChessGui {
    pub fn new(ctx: &mut Context) -> GameResult<Self> {
        let mut game_state = GameState::new();
        let assets = Assets::new(ctx)?;
        
        let board_bottom = BOARD_OFFSET_Y + (BOARD_SIZE as f32 * SQUARE_SIZE);
        let button_y = board_bottom + 120.0;
        
        let offer_draw_button = Button::new(
            BOARD_OFFSET_X, 
            button_y, 
            BUTTON_WIDTH, 
            BUTTON_HEIGHT, 
            "Offer Draw"
        );
        
        let accept_draw_button = Button::new(
            BOARD_OFFSET_X + BUTTON_WIDTH + BUTTON_MARGIN, 
            button_y, 
            BUTTON_WIDTH, 
            BUTTON_HEIGHT, 
            "Accept Draw"
        );
        
        let restart_button = Button::new(
            BOARD_OFFSET_X + (BUTTON_WIDTH + BUTTON_MARGIN) * 2.0, 
            button_y, 
            BUTTON_WIDTH, 
            BUTTON_HEIGHT, 
            "New Game"
        );
        
        let timer_button_y = button_y + BUTTON_HEIGHT + BUTTON_MARGIN;
        
        let timer_toggle_button = Button::new(
            BOARD_OFFSET_X, 
            timer_button_y, 
            BUTTON_WIDTH, 
            BUTTON_HEIGHT, 
            "Start Timer"
        );
        
        let timer_options = vec![
            (5, 0),   // 5 minutes, no increment (Blitz)
            (10, 0),  // 10 minutes, no increment (Rapid)
            (15, 10), // 15 minutes + 10 second increment (Rapid)
            (30, 0),  // 30 minutes, no increment (Classical)
            (60, 0),  // 60 minutes, no increment (Classical)
        ];
        
        let selected_timer_option = 1;
        let (minutes, increment) = timer_options[selected_timer_option];
        
        let timer_config_text = if increment > 0 {
            format!("{}m + {}s", minutes, increment)
        } else {
            format!("{} min", minutes)
        };
        
        let timer_config_button = Button::new(
            BOARD_OFFSET_X + BUTTON_WIDTH + BUTTON_MARGIN, 
            timer_button_y, 
            BUTTON_WIDTH, 
            BUTTON_HEIGHT, 
            &timer_config_text
        );
        
        // Add new coordinate toggle button
        let show_coordinates_button = Button::new(
            BOARD_OFFSET_X + (BUTTON_WIDTH + BUTTON_MARGIN) * 2.0, 
            timer_button_y, 
            BUTTON_WIDTH, 
            BUTTON_HEIGHT, 
            "Show Coords"
        );
        
        game_state.set_timers(minutes, increment);
        game_state.timer_enabled = false;
        
        Ok(Self {
            game_state,
            selected_square: None,
            possible_moves: Vec::new(),
            assets,
            offer_draw_button,
            accept_draw_button,
            restart_button,
            timer_toggle_button,
            timer_config_button,
            show_coordinates_button,
            show_square_coordinates: false,
            game_over: false,
            needs_redraw: true,
            timer_options,
            selected_timer_option,
            last_timer_update: Instant::now(),
        })
    }
    
    pub fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        if !self.needs_redraw {
            return Ok(());
        }
        
        let mut canvas = Canvas::from_frame(ctx, GgezColor::new(0.2, 0.2, 0.2, 1.0));
        
        self.draw_board(ctx, &mut canvas)?;
        
        self.draw_pieces(&mut canvas);
        
        self.draw_timers(ctx, &mut canvas)?;
        
        self.draw_status(&mut canvas)?;
        
        self.offer_draw_button.draw(ctx, &mut canvas)?;
        self.accept_draw_button.draw(ctx, &mut canvas)?;
        self.restart_button.draw(ctx, &mut canvas)?;
        self.timer_toggle_button.draw(ctx, &mut canvas)?;
        self.timer_config_button.draw(ctx, &mut canvas)?;
        self.show_coordinates_button.draw(ctx, &mut canvas)?;
        
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
                let x = BOARD_OFFSET_X + (file as f32) * SQUARE_SIZE;
                let y = BOARD_OFFSET_Y + (rank as f32) * SQUARE_SIZE;
                
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
                    let x = BOARD_OFFSET_X + (file as f32) * SQUARE_SIZE;
                    let y = BOARD_OFFSET_Y + (rank as f32) * SQUARE_SIZE;
                    
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
    
    fn draw_timers(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult<()> {
        // Calculate the center of the board width
        let board_width = BOARD_SIZE as f32 * SQUARE_SIZE;
        let center_x = BOARD_OFFSET_X + (board_width / 2.0);
        
        // Reduce timer dimensions
        let timer_width = TIMER_DISPLAY_WIDTH;  // No extra padding
        let timer_height = TIMER_DISPLAY_HEIGHT;
        
        // White timer (bottom) - Keep it closer to the board
        let white_time_text = self.game_state.white_timer.format_time();
        let white_timer_text = Text::new(format!("W: {}", white_time_text));  // Shortened label
        
        // Position the bottom timer closer to the board
        let bottom_timer_y_offset = 5.0;  // Reduced from 15.0
        
        // Create a smaller background rectangle for white timer
        let white_timer_rect = Rect::new(
            center_x - (timer_width / 2.0),
            BOARD_OFFSET_Y + board_width + bottom_timer_y_offset,
            timer_width,
            timer_height
        );
        
        let white_timer_bg = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            white_timer_rect,
            if self.game_state.current_turn == Color::White && 
               self.game_state.timer_enabled && !self.game_over {
                GgezColor::new(0.35, 0.35, 0.0, 0.8)
            } else {
                GgezColor::new(0.25, 0.25, 0.25, 0.8)
            }
        )?;
        
        // Draw white timer background
        canvas.draw(&white_timer_bg, DrawParam::default());
        
        // Add border around white timer background
        let white_timer_border = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::stroke(1.0),  // Thinner border
            white_timer_rect,
            if self.game_state.current_turn == Color::White && 
               self.game_state.timer_enabled && !self.game_over {
                GgezColor::YELLOW
            } else {
                GgezColor::new(0.6, 0.6, 0.6, 0.8)
            }
        )?;
        canvas.draw(&white_timer_border, DrawParam::default());
        
        // Display white timer text with smaller scale
        canvas.draw(
            &white_timer_text,
            DrawParam::default()
                .dest(Point2 {
                    x: center_x,
                    y: BOARD_OFFSET_Y + board_width + bottom_timer_y_offset + (timer_height / 2.0) - 2.0,
                })
                .offset(Point2 { x: 0.5, y: 0.5 })  // Center both horizontally and vertically
                .color(if self.game_state.current_turn == Color::White && 
                       self.game_state.timer_enabled && !self.game_over {
                    GgezColor::YELLOW
                } else {
                    GgezColor::WHITE
                })
                .scale(Vector2 { x: 1.0, y: 1.0 })  // Normal text size
        );
        
        // Black timer (top)
        let black_time_text = self.game_state.black_timer.format_time();
        let black_timer_text = Text::new(format!("B: {}", black_time_text));  // Shortened label
        
        // Position the top timer closer to the board
        let top_timer_y_offset = 5.0;  // Reduced from 35.0
        
        // Create a smaller background rectangle for black timer
        let black_timer_rect = Rect::new(
            center_x - (timer_width / 2.0),
            BOARD_OFFSET_Y - timer_height - top_timer_y_offset,
            timer_width,
            timer_height
        );
        
        let black_timer_bg = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            black_timer_rect,
            if self.game_state.current_turn == Color::Black && 
               self.game_state.timer_enabled && !self.game_over {
                GgezColor::new(0.35, 0.35, 0.0, 0.8)
            } else {
                GgezColor::new(0.25, 0.25, 0.25, 0.8)
            }
        )?;
        
        // Draw black timer background
        canvas.draw(&black_timer_bg, DrawParam::default());
        
        // Add border around black timer background
        let black_timer_border = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::stroke(1.0),  // Thinner border
            black_timer_rect,
            if self.game_state.current_turn == Color::Black && 
               self.game_state.timer_enabled && !self.game_over {
                GgezColor::YELLOW
            } else {
                GgezColor::new(0.6, 0.6, 0.6, 0.8)
            }
        )?;
        canvas.draw(&black_timer_border, DrawParam::default());
        
        // Display black timer text with smaller scale
        canvas.draw(
            &black_timer_text,
            DrawParam::default()
                .dest(Point2 {
                    x: center_x,
                    y: BOARD_OFFSET_Y - top_timer_y_offset - (timer_height / 2.0) - 2.0,
                })
                .offset(Point2 { x: 0.5, y: 0.5 })  // Center both horizontally and vertically
                .color(if self.game_state.current_turn == Color::Black && 
                       self.game_state.timer_enabled && !self.game_over {
                    GgezColor::YELLOW
                } else {
                    GgezColor::WHITE
                })
                .scale(Vector2 { x: 1.0, y: 1.0 })  // Normal text size
        );
        
        Ok(())
    }
    
    fn draw_status(&self, canvas: &mut Canvas) -> GameResult<()> {
        let mut status_text = format!("Current turn: {:?}", self.game_state.current_turn);
        
        if self.game_state.is_time_up() {
            // Determine which player lost on time
            if self.game_state.white_timer.remaining.as_secs() == 0 {
                status_text = "WHITE FLAG FALLEN - Black wins!".to_string();
            } else if self.game_state.black_timer.remaining.as_secs() == 0 {
                status_text = "BLACK FLAG FALLEN - White wins!".to_string();
            }
        } else if self.game_state.is_in_check(self.game_state.current_turn) {
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
        
        if let Some(color) = self.game_state.draw_offered {
            status_text = format!("{} - {:?} has offered a draw", status_text, color);
        }
        
        // Add time control display
        let (minutes, increment) = self.timer_options[self.selected_timer_option];
        let time_control_text = if increment > 0 {
            format!("Time control: {} min + {} sec", minutes, increment)
        } else {
            format!("Time control: {} min", minutes)
        };
        
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
        
        let time_control_display = Text::new(time_control_text);
        
        // Position time control text below halfmove clock
        canvas.draw(
            &time_control_display,
            DrawParam::default()
                .dest(Point2 {
                    x: BOARD_OFFSET_X,
                    y: BOARD_OFFSET_Y + (BOARD_SIZE as f32) * SQUARE_SIZE + 80.0,
                })
                .color(GgezColor::WHITE)
        );
        
        Ok(())
    }
    
    fn draw_promotion_dialog(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult<()> {
        if let Some(ref promotion) = self.game_state.promotion_pending {
            let (rank, file) = promotion.position;
            let color = promotion.color;
            
            let square_x = BOARD_OFFSET_X + (file as f32) * SQUARE_SIZE;
            let square_y = BOARD_OFFSET_Y + (rank as f32) * SQUARE_SIZE;
            
            let dialog_width = SQUARE_SIZE;
            let dialog_height = SQUARE_SIZE * 4.0; // Space for 4 pieces
            
            let dialog_y = if rank < 4 {
                square_y // Dialog extends downward
            } else {
                square_y - dialog_height + SQUARE_SIZE // Dialog extends upward
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
    
    pub fn handle_mouse_down(&mut self, button: MouseButton, x: f32, y: f32) -> GameResult<()> {
        if button == MouseButton::Left {
            let point = Point2 { x, y };
            
            if self.offer_draw_button.contains(point) {
                self.game_state.offer_draw(self.game_state.current_turn);
                self.needs_redraw = true;
                return Ok(());
            }
            
            if self.accept_draw_button.contains(point) {
                if self.game_state.accept_draw() {
                    self.game_over = true;
                    self.needs_redraw = true;
                    return Ok(());
                }
                return Ok(());
            }
            
            if self.restart_button.contains(point) {
                self.game_state = GameState::new();
                self.selected_square = None;
                self.possible_moves.clear();
                self.game_over = false;
                
                // Set up timers based on selected option
                let (minutes, increment) = self.timer_options[self.selected_timer_option];
                self.game_state.set_timers(minutes, increment);
                self.game_state.timer_enabled = false;
                
                // Update timer button text
                self.timer_toggle_button.text = "Start Timer".to_string();
                
                self.needs_redraw = true;
                return Ok(());
            }
            
            if self.timer_toggle_button.contains(point) {
                if self.game_state.timer_enabled {
                    println!("Disabling timers");
                    self.game_state.disable_timers();
                    self.timer_toggle_button.text = "Start Timer".to_string();
                } else {
                    println!("Enabling timers");
                    self.game_state.enable_timers();
                    self.timer_toggle_button.text = "Pause Timer".to_string();
                }
                self.needs_redraw = true;
                return Ok(());
            }
            
            if self.timer_config_button.contains(point) {
                // Cycle through time control options
                self.selected_timer_option = (self.selected_timer_option + 1) % self.timer_options.len();
                let (minutes, increment) = self.timer_options[self.selected_timer_option];
                
                // Update button text to show current time control
                if increment > 0 {
                    self.timer_config_button.text = format!("{}m + {}s", minutes, increment);
                } else {
                    self.timer_config_button.text = format!("{} min", minutes);
                }
                
                println!("Setting timers to {}m + {}s increment", minutes, increment);
                
                // Set new timers
                self.game_state.set_timers(minutes, increment);
                self.game_state.timer_enabled = false;
                self.timer_toggle_button.text = "Start Timer".to_string();
                
                self.needs_redraw = true;
                return Ok(());
            }
            
            // Handle click on the coordinate toggle button
            if self.show_coordinates_button.contains(point) {
                self.show_square_coordinates = !self.show_square_coordinates;
                
                // Update button text based on current state
                self.show_coordinates_button.text = if self.show_square_coordinates {
                    "Hide Coords".to_string()
                } else {
                    "Show Coords".to_string()
                };
                
                self.needs_redraw = true;
                return Ok(());
            }
            
            if let Some(ref promotion) = self.game_state.promotion_pending {
                let (rank, file) = promotion.position;
                
                let square_x = BOARD_OFFSET_X + (file as f32) * SQUARE_SIZE;
                let square_y = BOARD_OFFSET_Y + (rank as f32) * SQUARE_SIZE;
                
                let dialog_width = SQUARE_SIZE;
                let dialog_height = SQUARE_SIZE * 4.0; // Space for 4 pieces
                
                let dialog_y = if rank < 4 {
                    square_y // Dialog extends downward
                } else {
                    square_y - dialog_height + SQUARE_SIZE // Dialog extends upward
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
                        return Ok(());
                    }
                }
                
                return Ok(());
            }
            
            if self.game_over {
                return Ok(());
            }
            
            if x >= BOARD_OFFSET_X && y >= BOARD_OFFSET_Y {
                let file = ((x - BOARD_OFFSET_X) / SQUARE_SIZE) as usize;
                let rank = ((y - BOARD_OFFSET_Y) / SQUARE_SIZE) as usize;
                
                if file < BOARD_SIZE && rank < BOARD_SIZE {
                    if let Some(selected) = self.selected_square {
                        if self.possible_moves.contains(&(rank, file)) {
                            if self.game_state.make_move(selected, (rank, file)) {
                                self.selected_square = None;
                                self.possible_moves.clear();
                                
                                self.check_game_end();
                                
                                self.needs_redraw = true;
                                return Ok(());
                            }
                        }
                        
                        if selected == (rank, file) {
                            self.selected_square = None;
                            self.possible_moves.clear();
                            self.needs_redraw = true;
                            return Ok(());
                        }
                    }
                    
                    if let Some(piece) = self.game_state.board[rank][file] {
                        if piece.color == self.game_state.current_turn {
                            self.selected_square = Some((rank, file));
                            self.possible_moves = piece.get_possible_moves((rank, file), &self.game_state.board);
                            
                            self.possible_moves.retain(|&to_pos| {
                                let from_pos = self.selected_square.unwrap();
                                !self.game_state.would_be_in_check_after_move(from_pos, to_pos)
                            });
                            
                            self.needs_redraw = true;
                            return Ok(());
                        }
                    }
                    
                    if self.selected_square.is_some() {
                        self.selected_square = None;
                        self.possible_moves.clear();
                        self.needs_redraw = true;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    pub fn handle_mouse_move(&mut self, x: f32, y: f32) -> GameResult<()> {
        let point = Point2 { x, y };
        
        let hover_changed = 
            self.offer_draw_button.hovered != self.offer_draw_button.contains(point) ||
            self.accept_draw_button.hovered != self.accept_draw_button.contains(point) ||
            self.restart_button.hovered != self.restart_button.contains(point) ||
            self.timer_toggle_button.hovered != self.timer_toggle_button.contains(point) ||
            self.timer_config_button.hovered != self.timer_config_button.contains(point) ||
            self.show_coordinates_button.hovered != self.show_coordinates_button.contains(point);
        
        if hover_changed {
            self.offer_draw_button.set_hover(self.offer_draw_button.contains(point));
            self.accept_draw_button.set_hover(self.accept_draw_button.contains(point));
            self.restart_button.set_hover(self.restart_button.contains(point));
            self.timer_toggle_button.set_hover(self.timer_toggle_button.contains(point));
            self.timer_config_button.set_hover(self.timer_config_button.contains(point));
            self.show_coordinates_button.set_hover(self.show_coordinates_button.contains(point));
            self.needs_redraw = true;
        }
        
        Ok(())
    }
    
    pub fn update(&mut self) -> GameResult<()> {
        // Update the timer more frequently (every 16ms = ~60 FPS)
        if self.game_state.timer_enabled && !self.game_over {
            let now = Instant::now();
            if now.duration_since(self.last_timer_update) >= Duration::from_millis(16) {
                self.last_timer_update = now;
                
                // Get the current timer values before update
                let white_time_before = self.game_state.white_timer.remaining.as_secs();
                let black_time_before = self.game_state.black_timer.remaining.as_secs();
                
                // Update timers and check for time up
                if self.game_state.update_timers() {
                    println!("Time flag has fallen!");
                    self.game_over = true;
                    self.needs_redraw = true;  // Ensure we redraw when game ends
                }
                
                // Get timer values after update to see if they changed
                let white_time_after = self.game_state.white_timer.remaining.as_secs();
                let black_time_after = self.game_state.black_timer.remaining.as_secs();
                
                // Always force a redraw to ensure timer updates are visible
                if white_time_before != white_time_after || black_time_before != black_time_after {
                    self.needs_redraw = true;
                    println!("Timer updated - White: {}, Black: {}", 
                             self.game_state.white_timer.format_time(),
                             self.game_state.black_timer.format_time());
                }
            }
        }
        
        Ok(())
    }
    
    fn check_game_end(&mut self) {
        if self.game_state.is_checkmate() || 
           self.game_state.is_stalemate() || 
           self.game_state.is_draw() ||
           self.game_state.is_time_up() {
            self.game_over = true;
            self.needs_redraw = true;
        }
    }
} 
