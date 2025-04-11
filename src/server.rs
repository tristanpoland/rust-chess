use std::net::TcpListener;
use std::io::Write;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use serde_json;
use crate::network::{ChessClient, ClientRole, NetworkMessage, GameInfo, GameStatus};
use crate::board::GameState;
use crate::piece::{PieceType, Color};

const SERVER_VERSION: &str = "1.0.0";
const MAX_INACTIVE_TIME: Duration = Duration::from_secs(300); // 5 minutes
const GAME_CLEANUP_INTERVAL: Duration = Duration::from_secs(3600); // 1 hour

struct Game {
    id: String,
    host_name: String,
    white_client: Option<ChessClient>,
    black_client: Option<ChessClient>,
    spectators: HashMap<String, ChessClient>, // Map connection_id -> client
    game_state: GameState,
    status: GameStatus,
    created_at: u64,
    last_activity: SystemTime,
    chat_history: Vec<(String, String, bool)>, // (sender, message, is_spectator)
}

impl Game {
    fn new(id: String, host_name: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        Self {
            id,
            host_name,
            white_client: None,
            black_client: None,
            spectators: HashMap::new(),
            game_state: GameState::new(),
            status: GameStatus::Waiting,
            created_at: timestamp,
            last_activity: SystemTime::now(),
            chat_history: Vec::new(),
        }
    }

    fn broadcast_game_state(&mut self) -> Result<(), std::io::Error> {
        // Update last activity timestamp
        self.last_activity = SystemTime::now();
        
        let board_state = self.game_state.board.map(|row| {
            row.map(|cell| cell.map(|piece| (piece.piece_type, piece.color)))
        });

        let message = NetworkMessage::GameState {
            board: board_state,
            current_turn: self.game_state.current_turn,
            promotion_pending: self.game_state.promotion_pending.as_ref().map(|p| (p.position.0, p.position.1, p.color)),
            game_over: self.game_state.is_game_over(),
        };

        let serialized = format!("{}\n", serde_json::to_string(&message)?);
        
        // Send to white client
        if let Some(white_client) = &mut self.white_client {
            if let Some(stream) = &mut white_client.stream {
                if let Err(e) = stream.write_all(serialized.as_bytes()) {
                    println!("Error sending to white client: {}", e);
                    white_client.stream = None;
                }
            }
        }
        
        // Send to black client
        if let Some(black_client) = &mut self.black_client {
            if let Some(stream) = &mut black_client.stream {
                if let Err(e) = stream.write_all(serialized.as_bytes()) {
                    println!("Error sending to black client: {}", e);
                    black_client.stream = None;
                }
            }
        }
        
        // Send to all spectators
        let mut disconnected_spectators = Vec::new();
        for (id, spectator) in &mut self.spectators {
            if let Some(stream) = &mut spectator.stream {
                if let Err(e) = stream.write_all(serialized.as_bytes()) {
                    println!("Error sending to spectator: {}", e);
                    spectator.stream = None;
                    disconnected_spectators.push(id.clone());
                }
            } else {
                disconnected_spectators.push(id.clone());
            }
        }
        
        // Remove disconnected spectators
        for id in disconnected_spectators {
            self.spectators.remove(&id);
        }
        
        Ok(())
    }
    
    fn broadcast_message(&mut self, message: &NetworkMessage) -> Result<(), std::io::Error> {
        // Update last activity timestamp
        self.last_activity = SystemTime::now();
        
        let serialized = format!("{}\n", serde_json::to_string(message)?);
        
        // Send to white client
        if let Some(white_client) = &mut self.white_client {
            if let Some(stream) = &mut white_client.stream {
                if let Err(e) = stream.write_all(serialized.as_bytes()) {
                    println!("Error sending to white client: {}", e);
                    white_client.stream = None;
                }
            }
        }
        
        // Send to black client
        if let Some(black_client) = &mut self.black_client {
            if let Some(stream) = &mut black_client.stream {
                if let Err(e) = stream.write_all(serialized.as_bytes()) {
                    println!("Error sending to black client: {}", e);
                    black_client.stream = None;
                }
            }
        }
        
        // Send to all spectators
        let mut disconnected_spectators = Vec::new();
        for (id, spectator) in &mut self.spectators {
            if let Some(stream) = &mut spectator.stream {
                if let Err(e) = stream.write_all(serialized.as_bytes()) {
                    println!("Error sending to spectator: {}", e);
                    spectator.stream = None;
                    disconnected_spectators.push(id.clone());
                }
            } else {
                disconnected_spectators.push(id.clone());
            }
        }
        
        // Remove disconnected spectators
        for id in disconnected_spectators {
            self.spectators.remove(&id);
        }
        
        Ok(())
    }
    
    fn add_spectator(&mut self, mut spectator: ChessClient, name: String) -> Result<(), std::io::Error> {
        // Generate a unique spectator ID
        let spectator_id = Uuid::new_v4().to_string();
        
        // Set the role to spectator
        spectator.set_role(ClientRole::Spectator);
        
        // Notify others that a new spectator has joined
        let joined_message = NetworkMessage::SpectatorJoined { name: name.clone() };
        self.broadcast_message(&joined_message)?;
        
        // Add to chat history
        let system_message = format!("{} joined as spectator", name);
        self.chat_history.push(("System".to_string(), system_message.clone(), true));
        
        // Send current chat history to the new spectator
        if let Some(stream) = &mut spectator.stream {
            // First send the game state
            let board_state = self.game_state.board.map(|row| {
                row.map(|cell| cell.map(|piece| (piece.piece_type, piece.color)))
            });

            let state_message = NetworkMessage::GameState {
                board: board_state,
                current_turn: self.game_state.current_turn,
                promotion_pending: self.game_state.promotion_pending.as_ref()
                    .map(|p| (p.position.0, p.position.1, p.color)),
                game_over: self.game_state.is_game_over(),
            };
            
            let serialized = format!("{}\n", serde_json::to_string(&state_message)?);
            if let Err(e) = stream.write_all(serialized.as_bytes()) {
                return Err(e);
            }
            
            // Then send chat history
            for (sender, message, is_spectator) in &self.chat_history {
                let chat_message = NetworkMessage::ChatMessage {
                    sender: sender.clone(),
                    message: message.clone(),
                    is_spectator: *is_spectator,
                };
                
                let serialized = format!("{}\n", serde_json::to_string(&chat_message)?);
                if let Err(e) = stream.write_all(serialized.as_bytes()) {
                    return Err(e);
                }
            }
            
            // Send system message about joining
            let system_chat = NetworkMessage::ChatMessage {
                sender: "System".to_string(),
                message: system_message,
                is_spectator: true,
            };
            
            let serialized = format!("{}\n", serde_json::to_string(&system_chat)?);
            if let Err(e) = stream.write_all(serialized.as_bytes()) {
                return Err(e);
            }
        }
        
        // Add to spectator list
        self.spectators.insert(spectator_id, spectator);
        
        Ok(())
    }
    
    fn remove_spectator(&mut self, spectator_id: &str) -> Result<(), std::io::Error> {
        if let Some(spectator) = self.spectators.remove(spectator_id) {
            // Notify others that a spectator has left
            // We'd need to store spectator names to make this work properly
            let left_message = NetworkMessage::SpectatorLeft { 
                name: "Spectator".to_string() 
            };
            self.broadcast_message(&left_message)?;
        }
        
        Ok(())
    }
    
    fn handle_chat_message(&mut self, sender: String, message: String, is_spectator: bool) -> Result<(), std::io::Error> {
        // Add to chat history
        self.chat_history.push((sender.clone(), message.clone(), is_spectator));
        
        // Limit chat history size
        if self.chat_history.len() > 100 {
            self.chat_history.remove(0);
        }
        
        // Broadcast the message
        let chat_message = NetworkMessage::ChatMessage {
            sender,
            message,
            is_spectator,
        };
        
        self.broadcast_message(&chat_message)
    }
    
    fn handle_forfeit(&mut self, white_forfeits: bool) -> Result<(), std::io::Error> {
        let reason = if white_forfeits {
            "White player forfeited the game"
        } else {
            "Black player forfeited the game"
        };
        
        let end_message = NetworkMessage::GameEnd { reason: reason.to_string() };
        self.broadcast_message(&end_message)?;
        
        self.status = GameStatus::Completed;
        self.game_state.game_over = true;
        
        Ok(())
    }

    fn run(&mut self) -> Result<(), std::io::Error> {
        println!("Starting game: {}", self.id);
        
        // Send initial game state
        self.broadcast_game_state()?;

        // Start game loop
        let mut current_turn = true; // true for white, false for black
        
        self.status = GameStatus::InProgress;
        
        loop {
            // Process spectator messages first
            let mut disconnected_spectators = Vec::new();
            
            for (id, spectator) in &mut self.spectators {
                match spectator.receive_message() {
                    Ok(Some(NetworkMessage::ChatMessage { sender, message, is_spectator })) => {
                        // Forward chat message to all clients
                        if let Err(e) = self.handle_chat_message(sender, message, is_spectator) {
                            println!("Error handling chat message: {}", e);
                        }
                    },
                    Ok(Some(_)) => {
                        // Ignore other messages from spectators
                    },
                    Ok(None) => {
                        // No message, continue
                    },
                    Err(_) => {
                        // Connection error, mark for removal
                        disconnected_spectators.push(id.clone());
                    }
                }
            }
            
            // Remove disconnected spectators
            for id in &disconnected_spectators {
                self.spectators.remove(id);
            }
            
            // Check if both players are still connected
            let white_connected = self.white_client.as_ref().map_or(false, |c| c.stream.is_some());
            let black_connected = self.black_client.as_ref().map_or(false, |c| c.stream.is_some());
            
            if !white_connected && !black_connected && self.spectators.is_empty() {
                println!("All clients disconnected, ending game");
                self.status = GameStatus::Completed;
                break;
            }

            let sender = if current_turn {
                match self.white_client.as_mut() {
                    Some(client) if client.stream.is_some() => client,
                    _ => {
                        // White player disconnected or not available, skip turn
                        // If we've been waiting too long and have a black player, white forfeits
                        if black_connected && 
                           self.last_activity.elapsed().unwrap_or_default() > MAX_INACTIVE_TIME {
                            println!("White player inactive too long, forfeiting");
                            self.handle_forfeit(true)?; // true = white forfeits
                            break;
                        }
                        
                        // Just skip turn and keep waiting
                        println!("White player not available, waiting...");
                        thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                }
            } else {
                match self.black_client.as_mut() {
                    Some(client) if client.stream.is_some() => client,
                    _ => {
                        // Black player disconnected or not available
                        // If we've been waiting too long and have a white player, black forfeits
                        if white_connected && 
                           self.last_activity.elapsed().unwrap_or_default() > MAX_INACTIVE_TIME {
                            println!("Black player inactive too long, forfeiting");
                            self.handle_forfeit(false)?; // false = black forfeits
                            break;
                        }
                        
                        // Just skip turn and keep waiting
                        println!("Black player not available, waiting...");
                        thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                }
            };

            // Wait for move from current player
            match sender.receive_message() {
                Ok(Some(NetworkMessage::Move { from, to, promotion })) => {
                    let from = (from.0 as usize, from.1 as usize);
                    let to = (to.0 as usize, to.1 as usize);

                    // Apply the move to the server's game state
                    if self.game_state.make_move(from, to) {
                        if let Some(promotion) = promotion {
                            let piece_type = match promotion {
                                'Q' => PieceType::Queen,
                                'R' => PieceType::Rook,
                                'B' => PieceType::Bishop,
                                'N' => PieceType::Knight,
                                _ => {
                                    println!("Invalid promotion piece: {}", promotion);
                                    continue;
                                },
                            };
                            if !self.game_state.promote_pawn(piece_type) {
                                println!("Failed to promote pawn");
                                continue;
                            }
                        }

                        // Switch turns
                        current_turn = !current_turn;

                        // Broadcast updated game state to both clients
                        if let Err(e) = self.broadcast_game_state() {
                            println!("Error broadcasting game state: {}", e);
                        }
                    }
                }
                Ok(Some(NetworkMessage::ChatMessage { sender, message, is_spectator })) => {
                    // Handle chat message from player
                    if let Err(e) = self.handle_chat_message(sender, message, is_spectator) {
                        println!("Error handling chat message: {}", e);
                    }
                }
                Ok(Some(NetworkMessage::OfferDraw)) => {
                    // Forward draw offer to the other player
                    let draw_offer = NetworkMessage::DrawOffered;
                    let serialized = format!("{}\n", serde_json::to_string(&draw_offer)?);
                    
                    // Send to the non-current player
                    if current_turn {
                        // White is offering a draw, send to black
                        if let Some(black_client) = &mut self.black_client {
                            if let Some(stream) = &mut black_client.stream {
                                if let Err(e) = stream.write_all(serialized.as_bytes()) {
                                    println!("Error sending draw offer to black client: {}", e);
                                    black_client.stream = None;
                                }
                            }
                        }
                    } else {
                        // Black is offering a draw, send to white
                        if let Some(white_client) = &mut self.white_client {
                            if let Some(stream) = &mut white_client.stream {
                                if let Err(e) = stream.write_all(serialized.as_bytes()) {
                                    println!("Error sending draw offer to white client: {}", e);
                                    white_client.stream = None;
                                }
                            }
                        }
                    }
                    
                    // Log in chat
                    let player = if current_turn { "White" } else { "Black" };
                    self.handle_chat_message(
                        "System".to_string(),
                        format!("{} player offered a draw", player),
                        true
                    )?;
                }
                Ok(Some(NetworkMessage::AcceptDraw)) => {
                    // Forward draw acceptance to both players
                    let accept_draw = NetworkMessage::AcceptDraw;
                    let serialized = format!("{}\n", serde_json::to_string(&accept_draw)?);
                    
                    // Send to both players
                    if let Some(white_client) = &mut self.white_client {
                        if let Some(stream) = &mut white_client.stream {
                            if let Err(e) = stream.write_all(serialized.as_bytes()) {
                                println!("Error sending draw acceptance to white client: {}", e);
                                white_client.stream = None;
                            }
                        }
                    }
                    
                    if let Some(black_client) = &mut self.black_client {
                        if let Some(stream) = &mut black_client.stream {
                            if let Err(e) = stream.write_all(serialized.as_bytes()) {
                                println!("Error sending draw acceptance to black client: {}", e);
                                black_client.stream = None;
                            }
                        }
                    }
                    
                    // Log in chat
                    let player = if current_turn { "White" } else { "Black" };
                    self.handle_chat_message(
                        "System".to_string(),
                        format!("{} player accepted the draw offer", player),
                        true
                    )?;
                    
                    // End the game
                    let end_message = NetworkMessage::GameEnd { reason: "Draw agreed".to_string() };
                    let serialized = format!("{}\n", serde_json::to_string(&end_message)?);
                    
                    self.broadcast_message(&end_message)?;
                    
                    self.status = GameStatus::Completed;
                    self.game_state.game_over = true;
                    break;
                }
                Ok(Some(NetworkMessage::DeclineDraw)) => {
                    // Forward draw decline to the other player
                    let decline_draw = NetworkMessage::DeclineDraw;
                    let serialized = format!("{}\n", serde_json::to_string(&decline_draw)?);
                    
                    // Send to the non-current player (the one who offered the draw)
                    if !current_turn {
                        // White offered a draw, send decline to white
                        if let Some(white_client) = &mut self.white_client {
                            if let Some(stream) = &mut white_client.stream {
                                if let Err(e) = stream.write_all(serialized.as_bytes()) {
                                    println!("Error sending draw decline to white client: {}", e);
                                    white_client.stream = None;
                                }
                            }
                        }
                    } else {
                        // Black offered a draw, send decline to black
                        if let Some(black_client) = &mut self.black_client {
                            if let Some(stream) = &mut black_client.stream {
                                if let Err(e) = stream.write_all(serialized.as_bytes()) {
                                    println!("Error sending draw decline to black client: {}", e);
                                    black_client.stream = None;
                                }
                            }
                        }
                    }
                    
                    // Log in chat
                    let player = if current_turn { "White" } else { "Black" };
                    self.handle_chat_message(
                        "System".to_string(),
                        format!("{} player declined the draw offer", player),
                        true
                    )?;
                }
                Ok(Some(NetworkMessage::Resign)) => {
                    // Handle resignation
                    let resigner_color = if current_turn { "White" } else { "Black" };
                    let reason = format!("{} resigned", resigner_color);
                    
                    // Log in chat
                    self.handle_chat_message(
                        "System".to_string(),
                        format!("{} player resigned", resigner_color),
                        true
                    )?;
                    
                    // Forward resignation to all clients
                    let resign_message = NetworkMessage::Resign;
                    self.broadcast_message(&resign_message)?;
                    
                    // Send game end message to all
                    let end_message = NetworkMessage::GameEnd { reason };
                    self.broadcast_message(&end_message)?;
                    
                    self.status = GameStatus::Completed;
                    self.game_state.game_over = true;
                    break;
                }
                Ok(Some(NetworkMessage::RequestRematch)) => {
                    // Forward rematch request to the other player
                    let rematch_request = NetworkMessage::RequestRematch;
                    
                    if current_turn {
                        // White is requesting a rematch, send to black
                        if let Some(black_client) = &mut self.black_client {
                            if let Some(stream) = &mut black_client.stream {
                                if let Err(e) = stream.write_all(format!("{}\n", 
                                                serde_json::to_string(&rematch_request)?).as_bytes()) {
                                    println!("Error sending rematch request to black client: {}", e);
                                    black_client.stream = None;
                                }
                            }
                        }
                    } else {
                        // Black is requesting a rematch, send to white
                        if let Some(white_client) = &mut self.white_client {
                            if let Some(stream) = &mut white_client.stream {
                                if let Err(e) = stream.write_all(format!("{}\n", 
                                                serde_json::to_string(&rematch_request)?).as_bytes()) {
                                    println!("Error sending rematch request to white client: {}", e);
                                    white_client.stream = None;
                                }
                            }
                        }
                    }
                    
                    // Log in chat
                    let player = if current_turn { "White" } else { "Black" };
                    self.handle_chat_message(
                        "System".to_string(),
                        format!("{} player requested a rematch", player),
                        true
                    )?;
                }
                Ok(None) => {
                    // No message received, sleep briefly
                    thread::sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    println!("Error receiving message: {}", e);
                    // Handle disconnect
                    if current_turn {
                        if let Some(white_client) = &mut self.white_client {
                            white_client.stream = None;
                        }
                    } else {
                        if let Some(black_client) = &mut self.black_client {
                            black_client.stream = None;
                        }
                    }
                }
            }

            // Check if game is over
            if self.game_state.is_game_over() {
                let reason = if self.game_state.is_checkmate() {
                    if self.game_state.current_turn == Color::White {
                        "Black wins by checkmate"
                    } else {
                        "White wins by checkmate"
                    }
                } else if self.game_state.is_stalemate() {
                    "Draw by stalemate"
                } else if self.game_state.is_threefold_repetition() {
                    "Draw by threefold repetition"
                } else if self.game_state.is_fifty_move_rule() {
                    "Draw by fifty-move rule"
                } else if self.game_state.is_insufficient_material() {
                    "Draw by insufficient material"
                } else {
                    "Game over"
                };
                
                // Log in chat
                self.handle_chat_message(
                    "System".to_string(),
                    reason.to_string(),
                    true
                )?;
                
                let end_message = NetworkMessage::GameEnd { reason: reason.to_string() };
                self.broadcast_message(&end_message)?;
                
                self.status = GameStatus::Completed;
                break;
            }
        }

        Ok(())
    }
    
    fn reset_game(&mut self, swap_colors: bool) -> Result<(), std::io::Error> {
        // Reset the game state
        self.game_state = GameState::new();
        self.status = GameStatus::InProgress;
        
        // Clear chat history except for a system message about the new game
        self.chat_history.clear();
        self.chat_history.push((
            "System".to_string(), 
            "A new game has started".to_string(), 
            true
        ));
        
        // Optionally swap player colors
        if swap_colors {
            std::mem::swap(&mut self.white_client, &mut self.black_client);
        }
        
        // Notify clients about the new game and their colors
        if let Some(white_client) = &mut self.white_client {
            let message = NetworkMessage::RematchAccepted { is_white: true };
            if let Some(stream) = &mut white_client.stream {
                if let Err(e) = stream.write_all(format!("{}\n", serde_json::to_string(&message)?).as_bytes()) {
                    println!("Error sending rematch accepted to white client: {}", e);
                    white_client.stream = None;
                }
            }
            white_client.set_role(ClientRole::Player { is_white: true });
        }
        
        if let Some(black_client) = &mut self.black_client {
            let message = NetworkMessage::RematchAccepted { is_white: false };
            if let Some(stream) = &mut black_client.stream {
                if let Err(e) = stream.write_all(format!("{}\n", serde_json::to_string(&message)?).as_bytes()) {
                    println!("Error sending rematch accepted to black client: {}", e);
                    black_client.stream = None;
                }
            }
            black_client.set_role(ClientRole::Player { is_white: false });
        }
        
        // Send system message about new game to all spectators
        let new_game_message = NetworkMessage::ChatMessage {
            sender: "System".to_string(),
            message: "A new game has started".to_string(),
            is_spectator: true,
        };
        
        for (_id, spectator) in &mut self.spectators {
            if let Some(stream) = &mut spectator.stream {
                if let Err(e) = stream.write_all(format!("{}\n", serde_json::to_string(&new_game_message)?).as_bytes()) {
                    println!("Error sending new game message to spectator: {}", e);
                    spectator.stream = None;
                }
            }
        }
        
        // Send initial game state
        self.broadcast_game_state()?;
        
        Ok(())
    }
    
    fn spectator_count(&self) -> u8 {
        self.spectators.len() as u8
    }
    
    fn player_count(&self) -> u8 {
        let mut count = 0;
        if self.white_client.is_some() {
            count += 1;
        }
        if self.black_client.is_some() {
            count += 1;
        }
        count
    }
    
    fn is_inactive(&self) -> bool {
        // Calculate how long since last activity
        let elapsed = self.last_activity.elapsed().unwrap_or_default();
        
        // Remove games that have been inactive for too long or are completed and have no spectators
        (self.status == GameStatus::Completed && self.spectators.is_empty()) ||
        (elapsed > MAX_INACTIVE_TIME && self.spectators.is_empty())
    }
}

pub struct ChessServer {
    listener: TcpListener,
    games: Arc<Mutex<HashMap<String, Game>>>,
}

impl ChessServer {
    pub fn new(port: u16) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
        listener.set_nonblocking(true)?;
        println!("Chess server v{} started on port {}", SERVER_VERSION, port);
        
        Ok(Self { 
            listener,
            games: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn send_game_list(&self, client: &mut ChessClient) -> Result<(), std::io::Error> {
        let games = self.games.lock().unwrap();
        
        let game_infos: Vec<GameInfo> = games.values()
            .map(|game| GameInfo {
                game_id: game.id.clone(),
                host_name: game.host_name.clone(),
                status: game.status.clone(),
                player_count: game.player_count(),
                spectator_count: game.spectator_count(),
                created_at: game.created_at,
            })
            .collect();

        let message = NetworkMessage::GameList { available_games: game_infos };
        let serialized = format!("{}\n", serde_json::to_string(&message)?);
        
        if let Some(stream) = &mut client.stream {
            if let Err(e) = stream.write_all(serialized.as_bytes()) {
                println!("Error sending game list: {}", e);
                client.stream = None;
                return Err(e);
            }
        }
        
        Ok(())
    }
    
    fn cleanup_inactive_games(&self) {
        let mut games = self.games.lock().unwrap();
        let mut games_to_remove = Vec::new();
        
        // Identify inactive games
        for (game_id, game) in games.iter() {
            if game.is_inactive() {
                games_to_remove.push(game_id.clone());
            }
        }
        
        // Remove inactive games
        for game_id in games_to_remove {
            println!("Removing inactive game: {}", game_id);
            games.remove(&game_id);
        }
    }

    pub fn run(&mut self) -> Result<(), std::io::Error> {
        println!("Chess server started, waiting for connections...");
        
        let games_clone = Arc::clone(&self.games);
        
        // Start a thread for periodic cleanup of inactive games
        thread::spawn(move || {
            loop {
                thread::sleep(GAME_CLEANUP_INTERVAL);
                
                // Get a lock on the games map and clean up inactive games
                let mut games = games_clone.lock().unwrap();
                let mut games_to_remove = Vec::new();
                
                // Identify inactive games
                for (game_id, game) in games.iter() {
                    if game.is_inactive() {
                        games_to_remove.push(game_id.clone());
                    }
                }
                
                // Remove inactive games
                for game_id in &games_to_remove {
                    println!("Cleanup: Removing inactive game: {}", game_id);
                    games.remove(game_id);
                }
                
                println!("Cleanup: Removed {} inactive games. Active games: {}", 
                         games_to_remove.len(), games.len());
            }
        });
        
        loop {
            // Periodically clean up inactive games
            self.cleanup_inactive_games();
            
            match self.listener.accept() {
                Ok((stream, addr)) => {
                    println!("New connection from: {}", addr);
                    stream.set_nonblocking(true)?;
                    
                    let mut client = ChessClient::with_role(stream, ClientRole::Spectator, "");
                    
                    let games_clone = Arc::clone(&self.games);
                    
                    // Wait for initial message from client
                    let connected = true;
                    while connected {
                        match client.receive_message() {
                            Ok(Some(NetworkMessage::CreateGame { player_name })) => {
                                let game_id = Uuid::new_v4().to_string();
                                let mut game = Game::new(game_id.clone(), player_name);
                                
                                // First player is white
                                client.set_role(ClientRole::Player { is_white: true });
                                game.white_client = Some(client);
                                
                                // Send game created confirmation
                                let message = NetworkMessage::GameCreated { game_id: game_id.clone() };
                                if let Some(ref mut stream) = game.white_client.as_mut().unwrap().stream {
                                    if let Err(e) = stream.write_all(format!("{}\n", serde_json::to_string(&message)?).as_bytes()) {
                                        println!("Error sending game created confirmation: {}", e);
                                        break;
                                    }
                                }
                                
                                // Add game to list
                                let mut games = games_clone.lock().unwrap();
                                games.insert(game_id.clone(), game);
                                
                                // Start game thread
                                let games_for_thread = Arc::clone(&games_clone);
                                thread::spawn(move || {
                                    let game_id_clone = game_id.clone();
                                    
                                    // Wait until both players join
                                    loop {
                                        let run_game = {
                                            let games = games_for_thread.lock().unwrap();
                                            if let Some(game) = games.get(&game_id_clone) {
                                                game.white_client.is_some() && game.black_client.is_some()
                                            } else {
                                                // Game was removed
                                                false
                                            }
                                        };
                                        
                                        if run_game {
                                            break;
                                        }
                                        
                                        // Sleep to avoid busy waiting
                                        std::thread::sleep(std::time::Duration::from_millis(100));
                                    }
                                    
                                    // Send game start messages
                                    {
                                        let mut games = games_for_thread.lock().unwrap();
                                        if let Some(game) = games.get_mut(&game_id_clone) {
                                            if let Some(white_client) = &mut game.white_client {
                                                let message = NetworkMessage::GameStart { 
                                                    is_white: true, 
                                                    game_id: game_id_clone.clone() 
                                                };
                                                if let Some(stream) = &mut white_client.stream {
                                                    if let Err(e) = stream.write_all(format!("{}\n", 
                                                               serde_json::to_string(&message).unwrap()).as_bytes()) {
                                                        println!("Error sending game start to white client: {}", e);
                                                        white_client.stream = None;
                                                    }
                                                }
                                            }
                                            
                                            if let Some(black_client) = &mut game.black_client {
                                                let message = NetworkMessage::GameStart { 
                                                    is_white: false, 
                                                    game_id: game_id_clone.clone() 
                                                };
                                                if let Some(stream) = &mut black_client.stream {
                                                    if let Err(e) = stream.write_all(format!("{}\n", 
                                                               serde_json::to_string(&message).unwrap()).as_bytes()) {
                                                        println!("Error sending game start to black client: {}", e);
                                                        black_client.stream = None;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    
                                    // Loop for multiple games (to handle rematches)
                                    loop {
                                        // Run the game
                                        {
                                            let mut games = games_for_thread.lock().unwrap();
                                            if let Some(game) = games.get_mut(&game_id_clone) {
                                                if let Err(e) = game.run() {
                                                    println!("Error running game {}: {}", game_id_clone, e);
                                                    break;
                                                }
                                            } else {
                                                break;
                                            }
                                        }
                                        
                                        // Game is over, wait for rematch requests
                                        let mut rematch_requested = false;
                                        let mut rematch_accepted = false;
                                        
                                        // Wait for up to 60 seconds for a rematch request
                                        for _ in 0..600 { // 600 * 100ms = 60 seconds
                                            {
                                                let mut games = games_for_thread.lock().unwrap();
                                                if let Some(game) = games.get_mut(&game_id_clone) {
                                                    // Check if white requested rematch
                                                    if let Some(white_client) = &mut game.white_client {
                                                        if let Ok(Some(NetworkMessage::RequestRematch)) = white_client.receive_message() {
                                                            rematch_requested = true;
                                                            
                                                            // Forward to black
                                                            if let Some(black_client) = &mut game.black_client {
                                                                let message = NetworkMessage::RequestRematch;
                                                                if let Some(stream) = &mut black_client.stream {
                                                                    if let Err(e) = stream.write_all(format!("{}\n", 
                                                                          serde_json::to_string(&message).unwrap()).as_bytes()) {
                                                                        println!("Error sending rematch request to black client: {}", e);
                                                                        black_client.stream = None;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    
                                                    // Check if black requested rematch
                                                    if let Some(black_client) = &mut game.black_client {
                                                        if let Ok(Some(NetworkMessage::RequestRematch)) = black_client.receive_message() {
                                                            rematch_requested = true;
                                                            
                                                            // Forward to white
                                                            if let Some(white_client) = &mut game.white_client {
                                                                let message = NetworkMessage::RequestRematch;
                                                                if let Some(stream) = &mut white_client.stream {
                                                                    if let Err(e) = stream.write_all(format!("{}\n", 
                                                                          serde_json::to_string(&message).unwrap()).as_bytes()) {
                                                                        println!("Error sending rematch request to white client: {}", e);
                                                                        white_client.stream = None;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    
                                                    // Check if rematch was accepted
                                                    if let Some(white_client) = &mut game.white_client {
                                                        if let Ok(Some(NetworkMessage::AcceptDraw)) = white_client.receive_message() {
                                                            // Using AcceptDraw as a proxy for accepting rematch
                                                            rematch_accepted = true;
                                                        }
                                                    }
                                                    
                                                    if let Some(black_client) = &mut game.black_client {
                                                        if let Ok(Some(NetworkMessage::AcceptDraw)) = black_client.receive_message() {
                                                            // Using AcceptDraw as a proxy for accepting rematch
                                                            rematch_accepted = true;
                                                        }
                                                    }
                                                    
                                                    // If rematch accepted, reset the game with swapped colors
                                                    if rematch_accepted {
                                                        println!("Rematch accepted for game {}", game_id_clone);
                                                        if let Err(e) = game.reset_game(true) { // Swap colors for fairness
                                                            println!("Error resetting game {}: {}", game_id_clone, e);
                                                        }
                                                        break;
                                                    }
                                                } else {
                                                    // Game was removed
                                                    break;
                                                }
                                            }
                                            
                                            // Sleep to avoid busy waiting
                                            std::thread::sleep(std::time::Duration::from_millis(100));
                                        }
                                        
                                        // If no rematch was accepted, break the loop
                                        if !rematch_accepted {
                                            break;
                                        }
                                    }
                                });
                                
                                break;
                            },
                            Ok(Some(NetworkMessage::JoinGame { game_id, player_name })) => {
                                let mut games = games_clone.lock().unwrap();
                                
                                if let Some(game) = games.get_mut(&game_id) {
                                    if game.status == GameStatus::Waiting && game.black_client.is_none() {
                                        println!("{} joined game {}", player_name, game_id);
                                        
                                        // Second player is black
                                        client.set_role(ClientRole::Player { is_white: false });
                                        game.black_client = Some(client);
                                        
                                        // Add a system message to chat history
                                        game.chat_history.push((
                                            "System".to_string(),
                                            format!("{} joined as black", player_name),
                                            true
                                        ));
                                        
                                        break;
                                    } else {
                                        println!("Game {} is not available for joining", game_id);
                                    }
                                } else {
                                    println!("Game {} not found", game_id);
                                }
                            },
                            Ok(Some(NetworkMessage::SpectateGame { game_id, spectator_name })) => {
                                let mut games = games_clone.lock().unwrap();
                                
                                if let Some(game) = games.get_mut(&game_id) {
                                    println!("{} spectating game {}", spectator_name, game_id);
                                    
                                    // Set role to spectator
                                    client.set_role(ClientRole::Spectator);
                                    
                                    // Add the spectator to the game
                                    if let Err(e) = game.add_spectator(client, spectator_name.clone()) {
                                        println!("Error adding spectator to game {}: {}", game_id, e);
                                    }
                                    
                                    break;
                                } else {
                                    println!("Game {} not found for spectating", game_id);
                                }
                            },
                            Ok(Some(NetworkMessage::RequestGameList)) => {
                                if let Err(e) = self.send_game_list(&mut client) {
                                    println!("Error sending game list: {}", e);
                                    break;
                                }
                            },
                            Ok(Some(NetworkMessage::Heartbeat)) => {
                                // Respond to heartbeat with a heartbeat
                                let heartbeat = NetworkMessage::Heartbeat;
                                if let Some(stream) = &mut client.stream {
                                    let serialized = format!("{}\n", serde_json::to_string(&heartbeat)?);
                                    if let Err(e) = stream.write_all(serialized.as_bytes()) {
                                        println!("Error sending heartbeat: {}", e);
                                        break;
                                    }
                                }
                            },
                            Ok(None) => {
                                // No message received yet, wait
                                std::thread::sleep(std::time::Duration::from_millis(100));
                            },
                            Err(e) => {
                                println!("Error receiving message from new client: {}", e);
                                break;
                            },
                            _ => {
                                println!("Unexpected message from client");
                            }
                        }
                    }
                },
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No new connection, continue
                    std::thread::sleep(std::time::Duration::from_millis(100));
                },
                Err(e) => {
                    println!("Error accepting connection: {}", e);
                }
            }
        }
    }
}