use std::net::{TcpListener, TcpStream};
use std::io::Write;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use uuid::Uuid;
use serde_json;
use crate::network::{ChessClient, NetworkMessage, GameInfo, GameStatus};
use crate::board::GameState;
use crate::piece::{Color, PieceType};

struct Game {
    id: String,
    host_name: String,
    white_client: Option<ChessClient>,
    black_client: Option<ChessClient>,
    game_state: GameState,
    status: GameStatus,
}

impl Game {
    fn new(id: String, host_name: String) -> Self {
        Self {
            id,
            host_name,
            white_client: None,
            black_client: None,
            game_state: GameState::new(),
            status: GameStatus::Waiting,
        }
    }

    fn broadcast_game_state(&mut self) -> Result<(), std::io::Error> {
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
            // Check if both clients are still connected
            let white_connected = self.white_client.as_ref().map_or(false, |c| c.stream.is_some());
            let black_connected = self.black_client.as_ref().map_or(false, |c| c.stream.is_some());
            
            if !white_connected && !black_connected {
                println!("Both clients disconnected, ending game");
                self.status = GameStatus::Completed;
                break;
            }

            let sender = if current_turn {
                match self.white_client.as_mut() {
                    Some(client) if client.stream.is_some() => client,
                    _ => {
                        // White player disconnected or not available, skip turn
                        println!("White player not available, skipping turn");
                        current_turn = !current_turn;
                        continue;
                    }
                }
            } else {
                match self.black_client.as_mut() {
                    Some(client) if client.stream.is_some() => client,
                    _ => {
                        // Black player disconnected or not available, skip turn
                        println!("Black player not available, skipping turn");
                        current_turn = !current_turn;
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
                Ok(Some(NetworkMessage::GameStart { .. })) => {
                    // Ignore GameStart messages after initial setup
                    println!("Received unexpected GameStart message");
                }
                Ok(Some(NetworkMessage::GameState { .. })) => {
                    // Ignore GameState messages from clients
                    println!("Received unexpected GameState message");
                }
                Ok(Some(NetworkMessage::GameEnd { reason })) => {
                    // Forward game end to both players
                    let end_message = NetworkMessage::GameEnd { reason: reason.clone() };
                    let serialized = format!("{}\n", serde_json::to_string(&end_message)?);
                    
                    if let Some(white_client) = &mut self.white_client {
                        if let Some(stream) = &mut white_client.stream {
                            if let Err(e) = stream.write_all(serialized.as_bytes()) {
                                println!("Error sending game end to white client: {}", e);
                                white_client.stream = None;
                            }
                        }
                    }
                    
                    if let Some(black_client) = &mut self.black_client {
                        if let Some(stream) = &mut black_client.stream {
                            if let Err(e) = stream.write_all(serialized.as_bytes()) {
                                println!("Error sending game end to black client: {}", e);
                                black_client.stream = None;
                            }
                        }
                    }
                    self.status = GameStatus::Completed;
                    break;
                }
                Ok(Some(NetworkMessage::CreateGame { .. })) => {
                    // Ignore CreateGame messages during game
                    println!("Received unexpected CreateGame message");
                }
                Ok(Some(NetworkMessage::JoinGame { .. })) => {
                    // Ignore JoinGame messages during game
                    println!("Received unexpected JoinGame message");
                }
                Ok(Some(NetworkMessage::GameCreated { .. })) => {
                    // Ignore GameCreated messages during game
                    println!("Received unexpected GameCreated message");
                }
                Ok(Some(NetworkMessage::GameList { .. })) => {
                    // Ignore GameList messages during game
                    println!("Received unexpected GameList message");
                }
                Ok(Some(NetworkMessage::RequestGameList)) => {
                    // Ignore RequestGameList messages during game
                    println!("Received unexpected RequestGameList message");
                }
                Ok(None) => {
                    // No message received, continue
                }
                Err(e) => {
                    println!("Error receiving message: {}", e);
                    if e.kind() == std::io::ErrorKind::ConnectionAborted || 
                       e.kind() == std::io::ErrorKind::ConnectionReset {
                        println!("Client disconnected");
                        if current_turn {
                            // White disconnected
                            if let Some(white_client) = &mut self.white_client {
                                white_client.stream = None;
                            }
                        } else {
                            // Black disconnected
                            if let Some(black_client) = &mut self.black_client {
                                black_client.stream = None;
                            }
                        }
                    }
                }
            }

            // Check if game is over
            if self.game_state.is_game_over() {
                let reason = if self.game_state.is_checkmate() {
                    "Checkmate"
                } else if self.game_state.is_stalemate() {
                    "Stalemate"
                } else if self.game_state.is_threefold_repetition() {
                    "Threefold repetition"
                } else if self.game_state.is_fifty_move_rule() {
                    "Fifty-move rule"
                } else if self.game_state.is_insufficient_material() {
                    "Insufficient material"
                } else {
                    "Unknown"
                };
                
                let end_message = NetworkMessage::GameEnd { reason: reason.to_string() };
                let serialized = format!("{}\n", serde_json::to_string(&end_message)?);
                
                if let Some(white_client) = &mut self.white_client {
                    if let Some(stream) = &mut white_client.stream {
                        if let Err(e) = stream.write_all(serialized.as_bytes()) {
                            println!("Error sending game end to white client: {}", e);
                            white_client.stream = None;
                        }
                    }
                }
                
                if let Some(black_client) = &mut self.black_client {
                    if let Some(stream) = &mut black_client.stream {
                        if let Err(e) = stream.write_all(serialized.as_bytes()) {
                            println!("Error sending game end to black client: {}", e);
                            black_client.stream = None;
                        }
                    }
                }
                self.status = GameStatus::Completed;
                break;
            }
        }

        Ok(())
    }
}

pub struct ChessServer {
    listener: TcpListener,
    games: Arc<Mutex<HashMap<String, Game>>>,
}

impl ChessServer {
    pub fn new(port: u16) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
        Ok(Self { 
            listener,
            games: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn send_game_list(&self, mut client: &mut ChessClient) -> Result<(), std::io::Error> {
        let games = self.games.lock().unwrap();
        let game_infos: Vec<GameInfo> = games.values()
            .filter(|game| game.status == GameStatus::Waiting)
            .map(|game| GameInfo {
                game_id: game.id.clone(),
                host_name: game.host_name.clone(),
                status: game.status.clone(),
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

    pub fn run(&mut self) -> Result<(), std::io::Error> {
        println!("Chess server started, waiting for connections...");
        
        self.listener.set_nonblocking(true)?;
        
        loop {
            match self.listener.accept() {
                Ok((mut stream, addr)) => {
                    println!("New connection from: {}", addr);
                    stream.set_nonblocking(true)?;
                    
                    let mut client = ChessClient::with_color(stream, false, "");
                    
                    let games_clone = Arc::clone(&self.games);
                    
                    // Wait for initial message from client
                    let mut connected = true;
                    while connected {
                        match client.receive_message() {
                            Ok(Some(NetworkMessage::CreateGame { player_name })) => {
                                let game_id = Uuid::new_v4().to_string();
                                let mut game = Game::new(game_id.clone(), player_name);
                                
                                // First player is white
                                client.is_white = true;
                                game.white_client = Some(client);
                                
                                // Send game created confirmation
                                let message = NetworkMessage::GameCreated { game_id: game_id.clone() };
                                if let Some(ref mut stream) = game.white_client.as_mut().unwrap().stream {
                                    if let Err(e) = stream.write_all(format!("{}\n", serde_json::to_string(&message)?).as_bytes()) {
                                        println!("Error sending game created confirmation: {}", e);
                                        connected = false;
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
                                    let mut run_game = false;
                                    
                                    // Wait until both players join
                                    loop {
                                        {
                                            let games = games_for_thread.lock().unwrap();
                                            if let Some(game) = games.get(&game_id_clone) {
                                                if game.white_client.is_some() && game.black_client.is_some() {
                                                    run_game = true;
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
                                    
                                    if run_game {
                                        let mut games = games_for_thread.lock().unwrap();
                                        if let Some(game) = games.get_mut(&game_id_clone) {
                                            // Send game start messages
                                            if let Some(white_client) = &mut game.white_client {
                                                let message = NetworkMessage::GameStart { is_white: true, game_id: game_id_clone.clone() };
                                                if let Some(stream) = &mut white_client.stream {
                                                    if let Err(e) = stream.write_all(format!("{}\n", serde_json::to_string(&message).unwrap()).as_bytes()) {
                                                        println!("Error sending game start to white client: {}", e);
                                                        white_client.stream = None;
                                                    }
                                                }
                                            }
                                            
                                            if let Some(black_client) = &mut game.black_client {
                                                let message = NetworkMessage::GameStart { is_white: false, game_id: game_id_clone.clone() };
                                                if let Some(stream) = &mut black_client.stream {
                                                    if let Err(e) = stream.write_all(format!("{}\n", serde_json::to_string(&message).unwrap()).as_bytes()) {
                                                        println!("Error sending game start to black client: {}", e);
                                                        black_client.stream = None;
                                                    }
                                                }
                                            }
                                        }
                                        
                                        // Release the lock before running the game
                                        drop(games);
                                        
                                        // Actually run the game
                                        {
                                            let mut games = games_for_thread.lock().unwrap();
                                            if let Some(game) = games.get_mut(&game_id_clone) {
                                                if let Err(e) = game.run() {
                                                    println!("Error running game {}: {}", game_id_clone, e);
                                                }
                                            }
                                        }
                                        
                                        // Clean up completed game after some delay
                                        std::thread::sleep(std::time::Duration::from_secs(60));
                                        let mut games = games_for_thread.lock().unwrap();
                                        games.remove(&game_id_clone);
                                        println!("Game {} removed from active games", game_id_clone);
                                    }
                                });
                                
                                connected = false;
                                break;
                            },
                            Ok(Some(NetworkMessage::JoinGame { game_id, player_name })) => {
                                let mut games = games_clone.lock().unwrap();
                                
                                if let Some(game) = games.get_mut(&game_id) {
                                    if game.status == GameStatus::Waiting && game.black_client.is_none() {
                                        println!("{} joined game {}", player_name, game_id);
                                        
                                        // Second player is black
                                        client.is_white = false;
                                        game.black_client = Some(client);
                                        
                                        connected = false;
                                        break;
                                    } else {
                                        println!("Game {} is not available for joining", game_id);
                                    }
                                } else {
                                    println!("Game {} not found", game_id);
                                }
                            },
                            Ok(Some(NetworkMessage::RequestGameList)) => {
                                if let Err(e) = self.send_game_list(&mut client) {
                                    println!("Error sending game list: {}", e);
                                    connected = false;
                                    break;
                                }
                            },
                            Ok(None) => {
                                // No message received yet, wait
                                std::thread::sleep(std::time::Duration::from_millis(100));
                            },
                            Err(e) => {
                                println!("Error receiving message from new client: {}", e);
                                connected = false;
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