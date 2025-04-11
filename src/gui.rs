use ggez::{Context, GameResult};
use ggez::graphics::{self, Canvas, Color as GgezColor, DrawParam, Rect, Text};
use ggez::input::mouse::MouseButton;
use ggez::mint::{Point2, Vector2};

use crate::board::{GameState, BOARD_SIZE, PromotionState};
use crate::piece::{PieceType, Color, Piece};
use crate::network::{ChessClient, ClientRole, GameInfo, GameStatus, NetworkMessage};
use std::io::Write;
use std::time::{Duration, Instant};

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
const DIALOG_BG: GgezColor = GgezColor::new(0.2, 0.2, 0.2, 0.9);
const ACCEPT_BUTTON_BG: GgezColor = GgezColor::new(0.3, 0.6, 0.3, 1.0);
const ACCEPT_BUTTON_HOVER: GgezColor = GgezColor::new(0.4, 0.7, 0.4, 1.0);
const DECLINE_BUTTON_BG: GgezColor = GgezColor::new(0.6, 0.3, 0.3, 1.0);
const DECLINE_BUTTON_HOVER: GgezColor = GgezColor::new(0.7, 0.4, 0.4, 1.0);
const SPECTATOR_PANEL_BG: GgezColor = GgezColor::new(0.2, 0.2, 0.3, 0.9);
const CHAT_BG: GgezColor = GgezColor::new(0.2, 0.2, 0.2, 0.9);
const CHAT_INPUT_BG: GgezColor = GgezColor::new(0.3, 0.3, 0.3, 1.0);

const BUTTON_WIDTH: f32 = 120.0;
const BUTTON_HEIGHT: f32 = 30.0;
const BUTTON_MARGIN: f32 = 20.0;
const DIALOG_WIDTH: f32 = 300.0;
const DIALOG_HEIGHT: f32 = 150.0;
const DIALOG_BUTTON_WIDTH: f32 = 100.0;
const DIALOG_BUTTON_HEIGHT: f32 = 30.0;

// Constants for spectator panel
const SPECTATOR_PANEL_WIDTH: f32 = 200.0;
const SPECTATOR_PANEL_HEIGHT: f32 = 300.0;
const CHAT_HEIGHT: f32 = 200.0;
const MAX_CHAT_MESSAGES: usize = 10;

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

pub struct SpectatorPanel {
    rect: Rect,
    chat_rect: Rect,
    chat_input_rect: Rect,
    send_button: Button,
    chat_messages: Vec<(String, String, bool)>, // sender, message, is_spectator
    chat_input: String,
    spectator_list: Vec<String>,
}

impl SpectatorPanel {
    fn new(x: f32, y: f32) -> Self {
        let rect = Rect::new(x, y, SPECTATOR_PANEL_WIDTH, SPECTATOR_PANEL_HEIGHT);
        let chat_rect = Rect::new(x, y + SPECTATOR_PANEL_HEIGHT - CHAT_HEIGHT, 
                                  SPECTATOR_PANEL_WIDTH, CHAT_HEIGHT);
        let chat_input_rect = Rect::new(x, y + SPECTATOR_PANEL_HEIGHT - 30.0, 
                                         SPECTATOR_PANEL_WIDTH - 60.0, 25.0);
        let send_button = Button::new(
            x + SPECTATOR_PANEL_WIDTH - 55.0,
            y + SPECTATOR_PANEL_HEIGHT - 30.0,
            50.0,
            25.0,
            "Send"
        );
        
        Self {
            rect,
            chat_rect,
            chat_input_rect,
            send_button,
            chat_messages: Vec::new(),
            chat_input: String::new(),
            spectator_list: Vec::new(),
        }
    }
    
    fn add_chat_message(&mut self, sender: String, message: String, is_spectator: bool) {
        self.chat_messages.push((sender, message, is_spectator));
        
        // Limit the number of messages
        if self.chat_messages.len() > MAX_CHAT_MESSAGES {
            self.chat_messages.remove(0);
        }
    }
    
    fn add_spectator(&mut self, name: String) {
        self.spectator_list.push(name);
    }
    
    fn remove_spectator(&mut self, name: &str) {
        self.spectator_list.retain(|n| n != name);
    }
    
    fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult<()> {
        // Draw main panel background
        let panel_mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            self.rect,
            SPECTATOR_PANEL_BG,
        )?;
        canvas.draw(&panel_mesh, DrawParam::default());
        
        // Draw panel border
        let border_mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::stroke(2.0),
            self.rect,
            GgezColor::WHITE,
        )?;
        canvas.draw(&border_mesh, DrawParam::default());
        
        // Draw "Spectators" header
        let header_text = Text::new("Spectators");
        canvas.draw(
            &header_text,
            DrawParam::default()
                .dest(Point2 {
                    x: self.rect.x + 10.0,
                    y: self.rect.y + 10.0,
                })
                .color(GgezColor::WHITE)
        );
        
        // Draw spectator list
        for (i, name) in self.spectator_list.iter().enumerate() {
            let spectator_text = Text::new(name);
            canvas.draw(
                &spectator_text,
                DrawParam::default()
                    .dest(Point2 {
                        x: self.rect.x + 20.0,
                        y: self.rect.y + 40.0 + (i as f32 * 20.0),
                    })
                    .color(GgezColor::WHITE)
            );
        }
        
        // Draw chat area background
        let chat_mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            self.chat_rect,
            CHAT_BG,
        )?;
        canvas.draw(&chat_mesh, DrawParam::default());
        
        // Draw chat area border
        let chat_border = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::stroke(1.0),
            self.chat_rect,
            GgezColor::WHITE,
        )?;
        canvas.draw(&chat_border, DrawParam::default());
        
        // Draw "Chat" header
        let chat_header = Text::new("Chat");
        canvas.draw(
            &chat_header,
            DrawParam::default()
                .dest(Point2 {
                    x: self.chat_rect.x + 10.0,
                    y: self.chat_rect.y + 5.0,
                })
                .color(GgezColor::WHITE)
        );
        
        // Draw chat messages
        for (i, (sender, message, is_spectator)) in self.chat_messages.iter().enumerate() {
            let sender_color = if *is_spectator {
                GgezColor::new(0.7, 0.7, 1.0, 1.0) // Blue for spectators
            } else {
                GgezColor::new(1.0, 0.7, 0.7, 1.0) // Red for players
            };
            
            // Draw sender name
            let sender_text = Text::new(format!("{}: ", sender));
            canvas.draw(
                &sender_text,
                DrawParam::default()
                    .dest(Point2 {
                        x: self.chat_rect.x + 10.0,
                        y: self.chat_rect.y + 30.0 + (i as f32 * 20.0),
                    })
                    .color(sender_color)
            );
            
            // Calculate where message text should start after sender name
            let sender_width = 70.0; // Approximate width for sender name
            
            // Draw message text
            let message_text = Text::new(message);
            canvas.draw(
                &message_text,
                DrawParam::default()
                    .dest(Point2 {
                        x: self.chat_rect.x + 10.0 + sender_width,
                        y: self.chat_rect.y + 30.0 + (i as f32 * 20.0),
                    })
                    .color(GgezColor::WHITE)
            );
        }
        
        // Draw chat input background
        let input_mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            self.chat_input_rect,
            CHAT_INPUT_BG,
        )?;
        canvas.draw(&input_mesh, DrawParam::default());
        
        // Draw chat input border
        let input_border = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::stroke(1.0),
            self.chat_input_rect,
            GgezColor::WHITE,
        )?;
        canvas.draw(&input_border, DrawParam::default());
        
        // Draw chat input text
        let input_text = Text::new(&self.chat_input);
        canvas.draw(
            &input_text,
            DrawParam::default()
                .dest(Point2 {
                    x: self.chat_input_rect.x + 5.0,
                    y: self.chat_input_rect.y + self.chat_input_rect.h / 2.0,
                })
                .offset(Point2 { x: 0.0, y: 0.5 })
                .color(GgezColor::WHITE)
        );
        
        // Draw send button
        self.send_button.draw(ctx, canvas)?;
        
        Ok(())
    }
    
    fn contains_send_button(&self, point: Point2<f32>) -> bool {
        self.send_button.contains(point)
    }
    
    fn contains_input_field(&self, point: Point2<f32>) -> bool {
        self.chat_input_rect.contains(point)
    }
    
    fn handle_key_input(&mut self, key: char) {
        if key == '\u{08}' { // Backspace
            self.chat_input.pop();
        } else if !key.is_control() {
            self.chat_input.push(key);
        }
    }
    
    fn clear_input(&mut self) {
        self.chat_input.clear();
    }
    
    fn get_input(&self) -> &str {
        &self.chat_input
    }
}

pub struct ChessGui {
    game_state: GameState,
    selected_square: Option<(usize, usize)>,
    possible_moves: Vec<(usize, usize)>,
    assets: EmbeddedAssets,
    show_square_coordinates: bool,
    game_over: bool,
    needs_redraw: bool,
    is_network_game: bool,
    player_color: Option<Color>,
    network_client: Option<ChessClient>,
    game_id: Option<String>,
    player_name: String,
    available_games: Vec<GameInfo>,
    // Network buttons
    connect_button: Button,
    create_game_button: Button,
    refresh_games_button: Button,
    spectate_button: Button,
    join_game_buttons: Vec<Button>,
    // Game action buttons
    offer_draw_button: Button,
    resign_button: Button,
    rematch_button: Button,
    // Dialog state
    draw_offered: bool,
    rematch_offered: bool,
    // Button state
    server_address: String,
    show_game_list: bool,
    hovered_button: Option<usize>, // Index of button being hovered (0=connect, 1=create, 2=refresh, 3+=join game buttons)
    // Spectator mode
    is_spectator: bool,
    spectator_panel: SpectatorPanel,
    show_spectator_panel: bool,
    input_active: bool,
    last_heartbeat: Instant,
}

impl ChessGui {
    pub fn new(ctx: &mut Context) -> GameResult<Self> {
        let game_state = GameState::new();
        let assets = EmbeddedAssets::new(ctx)?;
        
        // Create network buttons
        let connect_button = Button::new(
            BOARD_OFFSET_X + (BOARD_SIZE as f32) * SQUARE_SIZE + BUTTON_MARGIN,
            BOARD_OFFSET_Y,
            BUTTON_WIDTH,
            BUTTON_HEIGHT,
            "Connect"
        );
        
        let create_game_button = Button::new(
            BOARD_OFFSET_X + (BOARD_SIZE as f32) * SQUARE_SIZE + BUTTON_MARGIN,
            BOARD_OFFSET_Y + BUTTON_HEIGHT + BUTTON_MARGIN,
            BUTTON_WIDTH,
            BUTTON_HEIGHT,
            "Create Game"
        );
        
        let refresh_games_button = Button::new(
            BOARD_OFFSET_X + (BOARD_SIZE as f32) * SQUARE_SIZE + BUTTON_MARGIN,
            BOARD_OFFSET_Y + 2.0 * (BUTTON_HEIGHT + BUTTON_MARGIN),
            BUTTON_WIDTH,
            BUTTON_HEIGHT,
            "Refresh Games"
        );
        
        let spectate_button = Button::new(
            BOARD_OFFSET_X + (BOARD_SIZE as f32) * SQUARE_SIZE + BUTTON_MARGIN,
            BOARD_OFFSET_Y + 3.0 * (BUTTON_HEIGHT + BUTTON_MARGIN),
            BUTTON_WIDTH,
            BUTTON_HEIGHT,
            "Spectate Game"
        );
        
        // Create game action buttons
        let offer_draw_button = Button::new(
            BOARD_OFFSET_X,
            BOARD_OFFSET_Y + (BOARD_SIZE as f32) * SQUARE_SIZE + 80.0,
            BUTTON_WIDTH,
            BUTTON_HEIGHT,
            "Offer Draw"
        );
        
        let resign_button = Button::new(
            BOARD_OFFSET_X + BUTTON_WIDTH + BUTTON_MARGIN,
            BOARD_OFFSET_Y + (BOARD_SIZE as f32) * SQUARE_SIZE + 80.0,
            BUTTON_WIDTH,
            BUTTON_HEIGHT,
            "Resign"
        );
        
        let rematch_button = Button::new(
            BOARD_OFFSET_X + 2.0 * (BUTTON_WIDTH + BUTTON_MARGIN),
            BOARD_OFFSET_Y + (BOARD_SIZE as f32) * SQUARE_SIZE + 80.0,
            BUTTON_WIDTH,
            BUTTON_HEIGHT,
            "Rematch"
        );
        
        // Create spectator panel
        let spectator_panel = SpectatorPanel::new(
            BOARD_OFFSET_X + (BOARD_SIZE as f32) * SQUARE_SIZE + BUTTON_MARGIN,
            BOARD_OFFSET_Y + 5.0 * (BUTTON_HEIGHT + BUTTON_MARGIN)
        );
        
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
            network_client: None,
            game_id: None,
            player_name: String::new(),
            available_games: Vec::new(),
            connect_button,
            create_game_button,
            refresh_games_button,
            spectate_button,
            join_game_buttons: Vec::new(),
            offer_draw_button,
            resign_button,
            rematch_button,
            draw_offered: false,
            rematch_offered: false,
            server_address: "localhost:8080".to_string(),
            show_game_list: false,
            hovered_button: None,
            is_spectator: false,
            spectator_panel,
            show_spectator_panel: false,
            input_active: false,
            last_heartbeat: Instant::now(),
        })
    }
    
    pub fn set_player_color(&mut self, is_white: bool) {
        self.player_color = Some(if is_white { Color::White } else { Color::Black });
        self.is_network_game = true;
        self.needs_redraw = true;
    }
    
    pub fn set_spectator_mode(&mut self, game_id: String) {
        self.is_spectator = true;
        self.is_network_game = true;
        self.game_id = Some(game_id);
        self.show_spectator_panel = true;
        self.needs_redraw = true;
    }
    
    pub fn handle_spectator_joined(&mut self, name: String) {
        self.spectator_panel.add_spectator(name.clone());
        self.spectator_panel.add_chat_message("System".to_string(), 
                                             format!("{} joined as spectator", name), 
                                             true);
        self.needs_redraw = true;
    }
    
    pub fn handle_spectator_left(&mut self, name: String) {
        self.spectator_panel.remove_spectator(&name);
        self.spectator_panel.add_chat_message("System".to_string(), 
                                             format!("{} left", name), 
                                             true);
        self.needs_redraw = true;
    }
    
    pub fn handle_chat_message(&mut self, sender: String, message: String, is_spectator: bool) {
        self.spectator_panel.add_chat_message(sender, message, is_spectator);
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
        
        // Draw network buttons in the right sidebar
        self.connect_button.draw(ctx, &mut canvas)?;
        
        if self.network_client.is_some() {
            self.create_game_button.draw(ctx, &mut canvas)?;
            self.refresh_games_button.draw(ctx, &mut canvas)?;
            self.spectate_button.draw(ctx, &mut canvas)?;
            
            // Draw connection status
            let connection_status = if self.network_client.as_ref().map_or(false, |c| c.is_connected()) {
                "Connected"
            } else {
                "Disconnected"
            };
            
            let status_text = Text::new(connection_status);
            canvas.draw(
                &status_text,
                DrawParam::default()
                    .dest(Point2 {
                        x: BOARD_OFFSET_X + (BOARD_SIZE as f32) * SQUARE_SIZE + BUTTON_MARGIN,
                        y: BOARD_OFFSET_Y + 4.0 * (BUTTON_HEIGHT + BUTTON_MARGIN),
                    })
                    .color(if connection_status == "Connected" { GgezColor::GREEN } else { GgezColor::RED })
            );
            
            // Draw spectator panel if enabled
            if self.show_spectator_panel {
                self.spectator_panel.draw(ctx, &mut canvas)?;
            }
            
            // Draw game list if it's visible
            if self.show_game_list {
                // Draw game list background
                let list_y = BOARD_OFFSET_Y + 5.0 * (BUTTON_HEIGHT + BUTTON_MARGIN);
                let list_width = BUTTON_WIDTH;
                let list_height = self.available_games.len() as f32 * (BUTTON_HEIGHT + 5.0);
                
                if !self.available_games.is_empty() {
                    let list_rect = Rect::new(
                        BOARD_OFFSET_X + (BOARD_SIZE as f32) * SQUARE_SIZE + BUTTON_MARGIN,
                        list_y,
                        list_width,
                        list_height
                    );
                    
                    let list_bg = graphics::Mesh::new_rectangle(
                        ctx,
                        graphics::DrawMode::fill(),
                        list_rect,
                        GgezColor::new(0.2, 0.2, 0.3, 1.0),
                    )?;
                    canvas.draw(&list_bg, DrawParam::default());
                    
                    // Draw game list items
                    for (_i, button) in self.join_game_buttons.iter().enumerate() {
                        button.draw(ctx, &mut canvas)?;
                    }
                } else {
                    // Draw "No games available" message
                    let no_games_text = Text::new("No games available");
                    canvas.draw(
                        &no_games_text,
                        DrawParam::default()
                            .dest(Point2 {
                                x: BOARD_OFFSET_X + (BOARD_SIZE as f32) * SQUARE_SIZE + BUTTON_MARGIN,
                                y: list_y,
                            })
                            .color(GgezColor::WHITE)
                    );
                }
            }
            
            // Draw game action buttons when appropriate
            if self.is_network_game && !self.is_spectator {
                if !self.game_over {
                    // During active game, show draw offer and resign buttons
                    self.offer_draw_button.draw(ctx, &mut canvas)?;
                    self.resign_button.draw(ctx, &mut canvas)?;
                } else {
                    // When game is over, show rematch button
                    self.rematch_button.draw(ctx, &mut canvas)?;
                }
                
                // If a draw has been offered to us, show dialog
                if self.draw_offered && !self.game_over {
                    self.draw_draw_offer_dialog(ctx, &mut canvas)?;
                }
                
                // If a rematch has been offered to us, show dialog
                if self.rematch_offered && self.game_over {
                    self.draw_rematch_offer_dialog(ctx, &mut canvas)?;
                }
            }
        }
        
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
        
        if self.is_spectator {
            status_text = format!("Spectating - Current turn: {:?}", self.game_state.current_turn);
        }
        
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
        
        // If in spectator mode, draw spectator indicator
        if self.is_spectator {
            let spectator_text = Text::new("SPECTATOR MODE");
            canvas.draw(
                &spectator_text,
                DrawParam::default()
                    .dest(Point2 {
                        x: BOARD_OFFSET_X,
                        y: BOARD_OFFSET_Y + (BOARD_SIZE as f32) * SQUARE_SIZE + 20.0,
                    })
                    .color(GgezColor::new(1.0, 0.7, 0.3, 1.0)) // Orange
            );
        }
        
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
    
    fn draw_draw_offer_dialog(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult<()> {
        // Create a semi-transparent background for the dialog
        let window_width = ctx.gfx.size().0;
        let window_height = ctx.gfx.size().1;
        
        let dialog_x = (window_width - DIALOG_WIDTH) / 2.0;
        let dialog_y = (window_height - DIALOG_HEIGHT) / 2.0;
        
        let dialog_rect = Rect::new(dialog_x, dialog_y, DIALOG_WIDTH, DIALOG_HEIGHT);
        let dialog_mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            dialog_rect,
            DIALOG_BG,
        )?;
        canvas.draw(&dialog_mesh, DrawParam::default());
        
        // Draw dialog border
        let border_mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::stroke(2.0),
            dialog_rect,
            GgezColor::WHITE,
        )?;
        canvas.draw(&border_mesh, DrawParam::default());
        
        // Draw dialog message
        let message_text = Text::new("Your opponent has offered a draw");
        canvas.draw(
            &message_text,
            DrawParam::default()
                .dest(Point2 {
                    x: dialog_x + DIALOG_WIDTH / 2.0,
                    y: dialog_y + 40.0,
                })
                .offset(Point2 { x: 0.5, y: 0.5 })
                .color(GgezColor::WHITE)
        );
        
        // Draw accept button
        let accept_rect = Rect::new(
            dialog_x + DIALOG_WIDTH / 2.0 - DIALOG_BUTTON_WIDTH - 10.0,
            dialog_y + DIALOG_HEIGHT - 50.0,
            DIALOG_BUTTON_WIDTH,
            DIALOG_BUTTON_HEIGHT
        );
        
        let accept_color = ACCEPT_BUTTON_BG; // Could add hover effect here
        
        let accept_mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            accept_rect,
            accept_color,
        )?;
        canvas.draw(&accept_mesh, DrawParam::default());
        
        let accept_border = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::stroke(2.0),
            accept_rect,
            GgezColor::WHITE,
        )?;
        canvas.draw(&accept_border, DrawParam::default());
        
        let accept_text = Text::new("Accept");
        canvas.draw(
            &accept_text,
            DrawParam::default()
                .dest(Point2 {
                    x: accept_rect.x + accept_rect.w / 2.0,
                    y: accept_rect.y + accept_rect.h / 2.0,
                })
                .offset(Point2 { x: 0.5, y: 0.5 })
                .color(GgezColor::WHITE)
        );
        
        // Draw decline button
        let decline_rect = Rect::new(
            dialog_x + DIALOG_WIDTH / 2.0 + 10.0,
            dialog_y + DIALOG_HEIGHT - 50.0,
            DIALOG_BUTTON_WIDTH,
            DIALOG_BUTTON_HEIGHT
        );
        
        let decline_color = DECLINE_BUTTON_BG; // Could add hover effect here
        
        let decline_mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            decline_rect,
            decline_color,
        )?;
        canvas.draw(&decline_mesh, DrawParam::default());
        
        let decline_border = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::stroke(2.0),
            decline_rect,
            GgezColor::WHITE,
        )?;
        canvas.draw(&decline_border, DrawParam::default());
        
        let decline_text = Text::new("Decline");
        canvas.draw(
            &decline_text,
            DrawParam::default()
                .dest(Point2 {
                    x: decline_rect.x + decline_rect.w / 2.0,
                    y: decline_rect.y + decline_rect.h / 2.0,
                })
                .offset(Point2 { x: 0.5, y: 0.5 })
                .color(GgezColor::WHITE)
        );
        
        Ok(())
    }

    fn draw_rematch_offer_dialog(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult<()> {
        // Create a semi-transparent background for the dialog
        let window_width = ctx.gfx.size().0;
        let window_height = ctx.gfx.size().1;
        
        let dialog_x = (window_width - DIALOG_WIDTH) / 2.0;
        let dialog_y = (window_height - DIALOG_HEIGHT) / 2.0;
        
        let dialog_rect = Rect::new(dialog_x, dialog_y, DIALOG_WIDTH, DIALOG_HEIGHT);
        let dialog_mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            dialog_rect,
            DIALOG_BG,
        )?;
        canvas.draw(&dialog_mesh, DrawParam::default());
        
        // Draw dialog border
        let border_mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::stroke(2.0),
            dialog_rect,
            GgezColor::WHITE,
        )?;
        canvas.draw(&border_mesh, DrawParam::default());
        
        // Draw dialog message
        let message_text = Text::new("Your opponent wants to play again");
        canvas.draw(
            &message_text,
            DrawParam::default()
                .dest(Point2 {
                    x: dialog_x + DIALOG_WIDTH / 2.0,
                    y: dialog_y + 40.0,
                })
                .offset(Point2 { x: 0.5, y: 0.5 })
                .color(GgezColor::WHITE)
        );
        
        // Draw accept button
        let accept_rect = Rect::new(
            dialog_x + DIALOG_WIDTH / 2.0 - DIALOG_BUTTON_WIDTH - 10.0,
            dialog_y + DIALOG_HEIGHT - 50.0,
            DIALOG_BUTTON_WIDTH,
            DIALOG_BUTTON_HEIGHT
        );
        
        let accept_color = ACCEPT_BUTTON_BG;
        
        let accept_mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            accept_rect,
            accept_color,
        )?;
        canvas.draw(&accept_mesh, DrawParam::default());
        
        let accept_border = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::stroke(2.0),
            accept_rect,
            GgezColor::WHITE,
        )?;
        canvas.draw(&accept_border, DrawParam::default());
        
        let accept_text = Text::new("Play Again");
        canvas.draw(
            &accept_text,
            DrawParam::default()
                .dest(Point2 {
                    x: accept_rect.x + accept_rect.w / 2.0,
                    y: accept_rect.y + accept_rect.h / 2.0,
                })
                .offset(Point2 { x: 0.5, y: 0.5 })
                .color(GgezColor::WHITE)
        );
        
        // Draw decline button
        let decline_rect = Rect::new(
            dialog_x + DIALOG_WIDTH / 2.0 + 10.0,
            dialog_y + DIALOG_HEIGHT - 50.0,
            DIALOG_BUTTON_WIDTH,
            DIALOG_BUTTON_HEIGHT
        );
        
        let decline_color = DECLINE_BUTTON_BG;
        
        let decline_mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            decline_rect,
            decline_color,
        )?;
        canvas.draw(&decline_mesh, DrawParam::default());
        
        let decline_border = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::stroke(2.0),
            decline_rect,
            GgezColor::WHITE,
        )?;
        canvas.draw(&decline_border, DrawParam::default());
        
        let decline_text = Text::new("No Thanks");
        canvas.draw(
            &decline_text,
            DrawParam::default()
                .dest(Point2 {
                    x: decline_rect.x + decline_rect.w / 2.0,
                    y: decline_rect.y + decline_rect.h / 2.0,
                })
                .offset(Point2 { x: 0.5, y: 0.5 })
                .color(GgezColor::WHITE)
        );
        
        Ok(())
    }
    
    pub fn handle_mouse_down(&mut self, button: MouseButton, x: f32, y: f32) -> GameResult<Option<MoveInfo>> {
        if button != MouseButton::Left {
            return Ok(None);
        }
        
        let point = Point2 { x, y };
        
        // Check for dialog button clicks first
        if self.draw_offered && !self.game_over {
            if self.handle_dialog_click(x, y, true)? {
                return Ok(None);
            }
        }
        
        if self.rematch_offered && self.game_over {
            if self.handle_dialog_click(x, y, false)? {
                return Ok(None);
            }
        }
        
        // Check if spectator panel is clicked
        if self.show_spectator_panel {
            if self.spectator_panel.contains_send_button(point) {
                // Send chat message
                if !self.spectator_panel.get_input().is_empty() {
                    if let Some(client) = &mut self.network_client {
                        let message = self.spectator_panel.get_input().to_string();
                        client.send_chat_message(message, self.player_name.clone())?;
                    }
                    self.spectator_panel.clear_input();
                    self.needs_redraw = true;
                }
                return Ok(None);
            }
            
            if self.spectator_panel.contains_input_field(point) {
                self.input_active = true;
                self.needs_redraw = true;
                return Ok(None);
            }
        }
        
        // Deactivate input field if clicking outside
        if self.input_active {
            self.input_active = false;
            self.needs_redraw = true;
        }
        
        // Check if a network button was clicked
        if self.connect_button.contains(point) {
            // Attempt to connect to server
            if self.network_client.is_none() {
                // Use a default player name if none is set
                let player_name = if self.player_name.is_empty() {
                    "Player".to_string()
                } else {
                    self.player_name.clone()
                };
                
                // Clone the server address to avoid borrowing issues
                let server_address = self.server_address.clone();
                if let Err(e) = self.init_network(&server_address, player_name) {
                    println!("Error connecting to server: {}", e);
                }
                self.needs_redraw = true;
                return Ok(None);
            }
        }
        
        // Check for spectate button
        if self.spectate_button.contains(point) && self.network_client.is_some() {
            // Show available games to spectate
            if let Err(e) = self.request_game_list() {
                println!("Error requesting game list: {}", e);
            }
            self.show_game_list = true;
            
            // Update the join game buttons to include spectating option
            self.update_join_game_buttons(true);
            
            self.needs_redraw = true;
            return Ok(None);
        }
        
        // Check for game action buttons
        if self.is_network_game && !self.is_spectator && self.network_client.is_some() {
            // Check game action buttons when in a network game
            if !self.game_over {
                if self.offer_draw_button.contains(point) {
                    if let Err(e) = self.offer_draw() {
                        println!("Error offering draw: {}", e);
                    }
                    self.needs_redraw = true;
                    return Ok(None);
                }
                
                if self.resign_button.contains(point) {
                    if let Err(e) = self.resign() {
                        println!("Error resigning: {}", e);
                    }
                    self.needs_redraw = true;
                    return Ok(None);
                }
            } else {
                if self.rematch_button.contains(point) {
                    if let Err(e) = self.request_rematch() {
                        println!("Error requesting rematch: {}", e);
                    }
                    self.needs_redraw = true;
                    return Ok(None);
                }
            }
        }
        
        if self.network_client.is_some() {
            if self.create_game_button.contains(point) {
                // Create a new game
                if let Err(e) = self.create_game() {
                    println!("Error creating game: {}", e);
                }
                self.needs_redraw = true;
                return Ok(None);
            }
            
            if self.refresh_games_button.contains(point) {
                // Refresh game list
                if let Err(e) = self.request_game_list() {
                    println!("Error refreshing game list: {}", e);
                }
                self.show_game_list = true;
                
                // Update the join game buttons
                self.update_join_game_buttons(false);
                
                self.needs_redraw = true;
                return Ok(None);
            }
            
            // Check if any join game button was clicked
            for (i, button) in self.join_game_buttons.iter().enumerate() {
                if button.contains(point) && i < self.available_games.len() {
                    let game_id = self.available_games[i].game_id.clone();
                    let game_status = self.available_games[i].status.clone();
                    
                    if self.is_spectator || button.text.starts_with("Spectate") {
                        // Spectate the game
                        if let Err(e) = self.spectate_game(game_id) {
                            println!("Error spectating game: {}", e);
                        }
                    } else {
                        // Join as a player
                        if let Err(e) = self.join_game(game_id) {
                            println!("Error joining game: {}", e);
                        }
                    }
                    
                    self.show_game_list = false;
                    self.needs_redraw = true;
                    return Ok(None);
                }
            }
        }

        if self.game_over || self.is_spectator {
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
                    
                    if self.is_network_game {
                        if self.game_state.promotion_pending.is_some() {
                            return Ok(Some(MoveInfo { from, to, promotion: None }));
                        }
                        
                        // This is a network game, send the move
                        self.send_move(from, to, None)?;
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
    
    pub fn handle_key_press(&mut self, key: char) -> GameResult<()> {
        if self.input_active {
            self.spectator_panel.handle_key_input(key);
            self.needs_redraw = true;
        }
        
        Ok(())
    }
    
    fn handle_dialog_click(&mut self, x: f32, y: f32, is_draw_dialog: bool) -> GameResult<bool> {
        // Get window dimensions from context size
        let window_width = 780.0; // Default window width from main.rs
        let window_height = 750.0; // Default window height from main.rs
        
        let dialog_x = (window_width - DIALOG_WIDTH) / 2.0;
        let dialog_y = (window_height - DIALOG_HEIGHT) / 2.0;
        
        // Check for accept button click
        let accept_rect = Rect::new(
            dialog_x + DIALOG_WIDTH / 2.0 - DIALOG_BUTTON_WIDTH - 10.0,
            dialog_y + DIALOG_HEIGHT - 50.0,
            DIALOG_BUTTON_WIDTH,
            DIALOG_BUTTON_HEIGHT
        );
        
        if x >= accept_rect.x && x < accept_rect.x + accept_rect.w && 
           y >= accept_rect.y && y < accept_rect.y + accept_rect.h {
            if is_draw_dialog {
                // Accept draw offer
                if let Some(client) = &mut self.network_client {
                    if let Err(e) = client.accept_draw() {
                        println!("Error accepting draw: {}", e);
                    }
                }
                self.draw_offered = false;
                self.game_over = true;
            } else {
                // Accept rematch offer
                if let Some(client) = &mut self.network_client {
                    if let Err(e) = client.accept_draw() { // Reusing accept_draw for now, ideally should be its own method
                        println!("Error accepting rematch: {}", e);
                    }
                }
                self.rematch_offered = false;
                // The server will send us a new game state
            }
            self.needs_redraw = true;
            return Ok(true);
        }
        
        // Check for decline button click
        let decline_rect = Rect::new(
            dialog_x + DIALOG_WIDTH / 2.0 + 10.0,
            dialog_y + DIALOG_HEIGHT - 50.0,
            DIALOG_BUTTON_WIDTH,
            DIALOG_BUTTON_HEIGHT
        );
        
        if x >= decline_rect.x && x < decline_rect.x + decline_rect.w && 
           y >= decline_rect.y && y < decline_rect.y + decline_rect.h {
            if is_draw_dialog {
                // Decline draw offer
                if let Some(client) = &mut self.network_client {
                    if let Err(e) = client.decline_draw() {
                        println!("Error declining draw: {}", e);
                    }
                }
                self.draw_offered = false;
            } else {
                // Decline rematch offer
                if let Some(client) = &mut self.network_client {
                    if let Err(e) = client.decline_draw() { // Reusing decline_draw for now
                        println!("Error declining rematch: {}", e);
                    }
                }
                self.rematch_offered = false;
            }
            self.needs_redraw = true;
            return Ok(true);
        }
        
        Ok(false)
    }
    
    pub fn handle_mouse_move(&mut self, x: f32, y: f32) -> GameResult<()> {
        let point = Point2 { x, y };
        let mut needs_redraw = false;
        
        // Reset all button hover states
        self.connect_button.set_hover(false);
        self.create_game_button.set_hover(false);
        self.refresh_games_button.set_hover(false);
        self.spectate_button.set_hover(false);
        self.offer_draw_button.set_hover(false);
        self.resign_button.set_hover(false);
        self.rematch_button.set_hover(false);
        
        if self.show_spectator_panel {
            self.spectator_panel.send_button.set_hover(false);
        }
        
        for button in &mut self.join_game_buttons {
            button.set_hover(false);
        }
        
        // Set hover state for the button under the mouse
        if self.connect_button.contains(point) {
            self.connect_button.set_hover(true);
            needs_redraw = true;
        } else if self.network_client.is_some() {
            if self.create_game_button.contains(point) {
                self.create_game_button.set_hover(true);
                needs_redraw = true;
            } else if self.refresh_games_button.contains(point) {
                self.refresh_games_button.set_hover(true);
                needs_redraw = true;
            } else if self.spectate_button.contains(point) {
                self.spectate_button.set_hover(true);
                needs_redraw = true;
            } else if self.is_network_game && !self.is_spectator {
                if !self.game_over {
                    if self.offer_draw_button.contains(point) {
                        self.offer_draw_button.set_hover(true);
                        needs_redraw = true;
                    } else if self.resign_button.contains(point) {
                        self.resign_button.set_hover(true);
                        needs_redraw = true;
                    }
                } else if self.rematch_button.contains(point) {
                    self.rematch_button.set_hover(true);
                    needs_redraw = true;
                }
            } else {
                for button in &mut self.join_game_buttons {
                    if button.contains(point) {
                        button.set_hover(true);
                        needs_redraw = true;
                        break;
                    }
                }
            }
            
            // Check spectator panel buttons
            if self.show_spectator_panel && self.spectator_panel.contains_send_button(point) {
                self.spectator_panel.send_button.set_hover(true);
                needs_redraw = true;
            }
        }
        
        if needs_redraw {
            self.needs_redraw = true;
        }
        
        Ok(())
    }
    
    pub fn update(&mut self) -> GameResult<()> {
        // Send heartbeat if needed (every 30 seconds)
        if let Some(client) = &mut self.network_client {
            if client.is_connected() && self.last_heartbeat.elapsed() > Duration::from_secs(30) {
                if let Err(e) = client.send_message(NetworkMessage::Heartbeat) {
                    println!("Error sending heartbeat: {}", e);
                } else {
                    self.last_heartbeat = Instant::now();
                }
            }
        }
        
        if self.is_network_game {
            self.handle_network_messages()?;
        }
        
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
                    
                    // If we're in a network game, send the promotion choice
                    if self.is_network_game {
                        let promotion_char = match selected_piece {
                            PieceType::Queen => 'Q',
                            PieceType::Rook => 'R',
                            PieceType::Bishop => 'B',
                            PieceType::Knight => 'N',
                            _ => panic!("Invalid promotion piece"),
                        };
                        
                        if let Some(client) = &mut self.network_client {
                            // The from/to positions were already sent, just need to send the promotion choice
                            if let Err(e) = client.send_move((0, 0), (0, 0), Some(promotion_char)) {
                                println!("Error sending promotion choice: {}", e);
                            }
                        }
                    }
                    
                    self.check_game_end();
                    
                    self.needs_redraw = true;
                }
            }
        }
        Ok(())
    }

    fn get_square_from_coords(&self, x: f32, y: f32) -> (usize, usize) {
        // Calculate the display coordinates from the screen position
        let display_file = ((x - BOARD_OFFSET_X) / SQUARE_SIZE) as isize;
        let display_rank = ((y - BOARD_OFFSET_Y) / SQUARE_SIZE) as isize;
        
        // Check if the coordinates are within the board bounds
        if display_file < 0 || display_file >= BOARD_SIZE as isize || 
           display_rank < 0 || display_rank >= BOARD_SIZE as isize {
            // If clicked outside the board, return a safe default position
            return (0, 0);
        }
        
        // Convert display coordinates to internal coordinates
        self.get_internal_coordinates(display_rank as usize, display_file as usize)
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
            // Ensure the result stays in bounds (0-7)
            let rank = if display_rank < BOARD_SIZE { 7 - display_rank } else { 0 };
            let file = if display_file < BOARD_SIZE { 7 - display_file } else { 0 };
            (rank, file)
        } else {
            // For white perspective: use coordinates as-is
            // Ensure the result stays in bounds (0-7)
            let rank = if display_rank < BOARD_SIZE { display_rank } else { 0 };
            let file = if display_file < BOARD_SIZE { display_file } else { 0 };
            (rank, file)
        }
    }

    // New method to initialize networking
    pub fn init_network(&mut self, server_address: &str, player_name: String) -> GameResult<()> {
        self.player_name = player_name;
        self.network_client = match ChessClient::new(server_address) {
            Ok(client) => Some(client),
            Err(e) => {
                println!("Failed to connect to server: {}", e);
                return Err(ggez::GameError::CustomError(format!("Network error: {}", e)));
            }
        };
        self.is_network_game = true;
        self.needs_redraw = true;
        Ok(())
    }

    pub fn create_game(&mut self) -> GameResult<()> {
        if let Some(client) = &mut self.network_client {
            // Create a new game
            let create_game = NetworkMessage::CreateGame { 
                player_name: self.player_name.clone() 
            };
            client.send_message(create_game)?;
            println!("Waiting for another player to join...");
        }
        Ok(())
    }

    pub fn join_game(&mut self, game_id: String) -> GameResult<()> {
        if let Some(client) = &mut self.network_client {
            // Join existing game
            let join_game = NetworkMessage::JoinGame { 
                game_id: game_id.clone(),
                player_name: self.player_name.clone() 
            };
            client.send_message(join_game)?;
            println!("Joining game {}...", game_id);
        }
        Ok(())
    }
    
    pub fn spectate_game(&mut self, game_id: String) -> GameResult<()> {
        if let Some(client) = &mut self.network_client {
            // Spectate existing game
            let spectate_game = NetworkMessage::SpectateGame { 
                game_id: game_id.clone(),
                spectator_name: self.player_name.clone() 
            };
            client.send_message(spectate_game)?;
            println!("Spectating game {}...", game_id);
            
            // Set spectator mode
            self.set_spectator_mode(game_id);
        }
        Ok(())
    }

    pub fn request_game_list(&mut self) -> GameResult<()> {
        if let Some(client) = &mut self.network_client {
            let request = NetworkMessage::RequestGameList;
            client.send_message(request)?;
        }
        Ok(())
    }

    pub fn get_available_games(&self) -> &Vec<GameInfo> {
        &self.available_games
    }

    pub fn handle_network_messages(&mut self) -> GameResult<()> {
        if let Some(client) = &mut self.network_client {
            if !client.is_connected() {
                println!("Attempting to reconnect...");
                if let Err(e) = client.reconnect() {
                    println!("Failed to reconnect: {}", e);
                    return Ok(());
                }
            }
    
            match client.receive_message() {
                Ok(Some(NetworkMessage::Move { from, to, promotion })) => {
                    self.handle_network_move(from, to, promotion)?;
                }
                Ok(Some(NetworkMessage::GameStart { is_white, game_id })) => {
                    self.set_player_color(is_white);
                    self.game_id = Some(game_id.clone());
                    self.is_spectator = false;
                    println!("Game started! You are playing as {}", if is_white { "white" } else { "black" });
                }
                Ok(Some(NetworkMessage::GameState { board, current_turn, promotion_pending, game_over })) => {
                    self.update_game_state(board, current_turn, promotion_pending, game_over)?;
                }
                Ok(Some(NetworkMessage::GameEnd { reason })) => {
                    println!("Game ended: {}", reason);
                    self.game_over = true;
                    self.needs_redraw = true;
                    
                    // Add system message to chat if spectator panel is active
                    if self.show_spectator_panel {
                        self.spectator_panel.add_chat_message(
                            "System".to_string(),
                            format!("Game ended: {}", reason),
                            true
                        );
                    }
                }
                Ok(Some(NetworkMessage::GameCreated { game_id })) => {
                    self.game_id = Some(game_id.clone());
                    println!("Game created with ID: {}", game_id);
                    println!("Waiting for an opponent to join...");
                }
                Ok(Some(NetworkMessage::GameList { available_games })) => {
                    self.available_games = available_games;
                    println!("Available games:");
                    for (i, game) in self.available_games.iter().enumerate() {
                        println!("{}. {} (hosted by {}) - Status: {:?}, Players: {}, Spectators: {}", 
                                 i + 1, game.game_id, game.host_name, game.status,
                                 game.player_count, game.spectator_count);
                    }
                    // Update join game buttons after receiving new game list
                    self.update_join_game_buttons(false);
                    self.needs_redraw = true;
                }
                Ok(Some(NetworkMessage::DrawOffered)) => {
                    println!("Your opponent has offered a draw");
                    if !self.is_spectator {
                        self.draw_offered = true;
                    }
                    
                    // Add to chat if spectator panel is active
                    if self.show_spectator_panel {
                        self.spectator_panel.add_chat_message(
                            "System".to_string(),
                            "Draw has been offered".to_string(),
                            true
                        );
                    }
                    
                    self.needs_redraw = true;
                }
                Ok(Some(NetworkMessage::AcceptDraw)) => {
                    println!("Your opponent has accepted your draw offer");
                    self.game_over = true;
                    
                    // Add to chat if spectator panel is active
                    if self.show_spectator_panel {
                        self.spectator_panel.add_chat_message(
                            "System".to_string(),
                            "Draw offer accepted".to_string(),
                            true
                        );
                    }
                    
                    self.needs_redraw = true;
                }
                Ok(Some(NetworkMessage::DeclineDraw)) => {
                    println!("Your opponent has declined your draw offer");
                    
                    // Add to chat if spectator panel is active
                    if self.show_spectator_panel {
                        self.spectator_panel.add_chat_message(
                            "System".to_string(),
                            "Draw offer declined".to_string(),
                            true
                        );
                    }
                    
                    self.needs_redraw = true;
                }
                Ok(Some(NetworkMessage::Resign)) => {
                    println!("Your opponent has resigned");
                    self.game_over = true;
                    
                    // Add to chat if spectator panel is active
                    if self.show_spectator_panel {
                        self.spectator_panel.add_chat_message(
                            "System".to_string(),
                            "A player has resigned".to_string(),
                            true
                        );
                    }
                    
                    self.needs_redraw = true;
                }
                Ok(Some(NetworkMessage::RequestRematch)) => {
                    println!("Your opponent wants to play again");
                    if !self.is_spectator {
                        self.rematch_offered = true;
                    }
                    self.needs_redraw = true;
                }
                Ok(Some(NetworkMessage::RematchAccepted { is_white })) => {
                    println!("Rematch accepted! You are playing as {}", if is_white { "white" } else { "black" });
                    // The server will send a new GameState message to set up the board
                    self.game_over = false;
                    self.set_player_color(is_white);
                    self.needs_redraw = true;
                }
                Ok(Some(NetworkMessage::SpectatorJoined { name })) => {
                    println!("Spectator joined: {}", name);
                    self.handle_spectator_joined(name);
                }
                Ok(Some(NetworkMessage::SpectatorLeft { name })) => {
                    println!("Spectator left: {}", name);
                    self.handle_spectator_left(name);
                }
                Ok(Some(NetworkMessage::ChatMessage { sender, message, is_spectator })) => {
                    println!("Chat: {}{}: {}", 
                             if is_spectator { "[Spectator] " } else { "" }, 
                             sender, message);
                    self.handle_chat_message(sender, message, is_spectator);
                }
                Ok(Some(NetworkMessage::Heartbeat)) => {
                    // Heartbeat received, update last heartbeat time
                    self.last_heartbeat = Instant::now();
                }
                Ok(Some(NetworkMessage::CreateGame { .. })) => {
                    // Ignore unexpected CreateGame messages from server
                    println!("Received unexpected CreateGame message");
                }
                Ok(Some(NetworkMessage::JoinGame { .. })) => {
                    // Ignore unexpected JoinGame messages from server
                    println!("Received unexpected JoinGame message");
                }
                Ok(Some(NetworkMessage::RequestGameList)) => {
                    // Ignore unexpected RequestGameList messages from server
                    println!("Received unexpected RequestGameList message");
                }
                Ok(Some(NetworkMessage::OfferDraw)) => {
                    // Ignore unexpected OfferDraw messages from server - should receive DrawOffered instead
                    println!("Received unexpected direct OfferDraw message");
                }
                Ok(Some(NetworkMessage::SpectateGame { .. })) => {
                    // Ignore unexpected SpectateGame messages from server
                    println!("Received unexpected SpectateGame message");
                }
                Ok(Some(NetworkMessage::ConnectionStatus { .. })) => {
                    // Handle connection status updates if needed
                    println!("Received connection status update");
                }
                Ok(None) => {
                    // No message received, continue
                }
                Err(e) => {
                    println!("Network error: {}", e);
                }
            }
        }
        Ok(())
    }


    pub fn send_move(&mut self, from: (u8, u8), to: (u8, u8), promotion: Option<char>) -> GameResult<()> {
        if let Some(client) = &mut self.network_client {
            if !client.is_connected() {
                println!("Cannot send move - not connected to server");
                return Ok(());
            }
            if let Err(e) = client.send_move(from, to, promotion) {
                println!("Error sending move: {}", e);
            }
        }
        Ok(())
    }

    // New method to update join game buttons based on available games
    fn update_join_game_buttons(&mut self, for_spectating: bool) {
        self.join_game_buttons.clear();
        
        let base_y = BOARD_OFFSET_Y + 5.0 * (BUTTON_HEIGHT + BUTTON_MARGIN);
        
        for (i, game) in self.available_games.iter().enumerate() {
            let y = base_y + i as f32 * (BUTTON_HEIGHT + 5.0);
            
            let button_text = if for_spectating {
                format!("Spectate: {}", game.host_name)
            } else if game.status == GameStatus::Waiting {
                format!("Join: {}", game.host_name)
            } else {
                format!("Spectate: {}", game.host_name)
            };
            
            let button = Button::new(
                BOARD_OFFSET_X + (BOARD_SIZE as f32) * SQUARE_SIZE + BUTTON_MARGIN,
                y,
                BUTTON_WIDTH,
                BUTTON_HEIGHT,
                &button_text
            );
            
            self.join_game_buttons.push(button);
        }
    }

    pub fn set_server_address(&mut self, address: String) {
        self.server_address = address;
    }
    
    pub fn get_server_address(&self) -> &str {
        &self.server_address
    }

    pub fn offer_draw(&mut self) -> GameResult<()> {
        if let Some(client) = &mut self.network_client {
            if !client.is_connected() {
                println!("Cannot offer draw - not connected to server");
                return Ok(());
            }
            if let Err(e) = client.offer_draw() {
                println!("Error offering draw: {}", e);
            }
        }
        Ok(())
    }
    
    pub fn resign(&mut self) -> GameResult<()> {
        if let Some(client) = &mut self.network_client {
            if !client.is_connected() {
                println!("Cannot resign - not connected to server");
                return Ok(());
            }
            if let Err(e) = client.resign() {
                println!("Error resigning: {}", e);
            } else {
                // Set game as over immediately - don't wait for server confirmation
                self.game_over = true;
                self.needs_redraw = true;
            }
        }
        Ok(())
    }
    
    pub fn request_rematch(&mut self) -> GameResult<()> {
        if let Some(client) = &mut self.network_client {
            if !client.is_connected() {
                println!("Cannot request rematch - not connected to server");
                return Ok(());
            }
            if let Err(e) = client.request_rematch() {
                println!("Error requesting rematch: {}", e);
            }
        }
        Ok(())
    }
}