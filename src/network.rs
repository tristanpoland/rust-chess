use std::net::{TcpStream, TcpListener};
use std::io::{Read, Write, ErrorKind};
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use crate::piece::{PieceType, Color};

// Timeout values
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const RECONNECT_ATTEMPTS: u32 = 3;

#[derive(Serialize, Deserialize, Debug)]
pub enum NetworkMessage {
    Move {
        from: (u8, u8),
        to: (u8, u8),
        promotion: Option<char>,
    },
    GameStart {
        is_white: bool,
        game_id: String,
        opponent_name: String,
    },
    GameEnd {
        reason: String,
    },
    GameState {
        board: [[Option<(PieceType, Color)>; 8]; 8],
        current_turn: Color,
        promotion_pending: Option<(usize, usize, Color)>,
        game_over: bool,
    },
    CreateGame {
        player_name: String,
    },
    JoinGame {
        game_id: String,
        player_name: String,
    },
    SpectateGame {
        game_id: String,
        spectator_name: String,
    },
    GameCreated {
        game_id: String,
    },
    GameList {
        available_games: Vec<GameInfo>,
    },
    RequestGameList,
    OfferDraw,
    AcceptDraw,
    DeclineDraw,
    Resign,
    RequestRematch,
    RematchAccepted {
        is_white: bool,
    },
    DrawOffered,
    // Heartbeat to keep connection alive
    Heartbeat,
    // Chat messages for spectators and players
    ChatMessage {
        sender: String,
        message: String,
        is_spectator: bool,
    },
    // Spectator notifications
    SpectatorJoined {
        name: String,
    },
    SpectatorLeft {
        name: String,
    },
    // Connection status update
    ConnectionStatus {
        connected: bool,
        message: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameInfo {
    pub game_id: String,
    pub host_name: String,
    pub status: GameStatus,
    pub player_count: Option<u8>, // Make it optional
    pub spectator_count: u8,
    pub created_at: u64, // timestamp
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum GameStatus {
    Waiting,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClientRole {
    Player { is_white: bool },
    Spectator,
}

pub struct ChessClient {
    pub stream: Option<TcpStream>,
    pub role: ClientRole,
    buffer: Vec<u8>,
    server_address: String,
    last_heartbeat: Instant,
    connection_id: String,
    pub player_name: String,
}

impl ChessClient {
    pub fn new(server_address: &str) -> Result<Self, std::io::Error> {
        let stream = Self::connect_with_timeout(server_address, CONNECTION_TIMEOUT)?;
        stream.set_nonblocking(true)?;
        
        let connection_id = uuid::Uuid::new_v4().to_string();
        
        Ok(Self {
            stream: Some(stream),
            role: ClientRole::Spectator, // Default role until assigned
            buffer: Vec::new(),
            server_address: server_address.to_string(),
            last_heartbeat: Instant::now(),
            connection_id,
            player_name: String::new(),
        })
    }

    fn connect_with_timeout(addr: &str, timeout: Duration) -> Result<TcpStream, std::io::Error> {
        use std::net::ToSocketAddrs;
        
        let addrs = addr.to_socket_addrs()?;
        let mut last_err = None;
        
        for addr in addrs {
            match TcpStream::connect_timeout(&addr, timeout) {
                Ok(stream) => return Ok(stream),
                Err(e) => last_err = Some(e),
            }
        }
        
        Err(last_err.unwrap_or_else(|| {
            std::io::Error::new(ErrorKind::AddrNotAvailable, "Could not resolve address")
        }))
    }

    pub fn with_role(stream: TcpStream, role: ClientRole, server_address: &str) -> Self {
        let connection_id = uuid::Uuid::new_v4().to_string();
        
        Self {
            stream: Some(stream),
            role,
            buffer: Vec::new(),
            server_address: server_address.to_string(),
            last_heartbeat: Instant::now(),
            connection_id,
            player_name: String::new(),
        }
    }

    pub fn reconnect(&mut self) -> Result<(), std::io::Error> {
        println!("Attempting to reconnect to server...");
        
        for attempt in 1..=RECONNECT_ATTEMPTS {
            match Self::connect_with_timeout(&self.server_address, CONNECTION_TIMEOUT) {
                Ok(stream) => {
                    stream.set_nonblocking(true)?;
                    self.stream = Some(stream);
                    self.last_heartbeat = Instant::now();
                    
                    println!("Successfully reconnected to server (attempt {}/{})", 
                             attempt, RECONNECT_ATTEMPTS);
                    
                    // Send reconnection message with connection ID
                    let reconnect_msg = NetworkMessage::ConnectionStatus {
                        connected: true,
                        message: format!("Reconnected client {}", self.connection_id),
                    };
                    
                    let serialized = serde_json::to_string(&reconnect_msg)?;
                    if let Some(stream) = &mut self.stream {
                        stream.write_all(format!("{}\n", serialized).as_bytes())?;
                    }
                    
                    return Ok(());
                }
                Err(e) => {
                    println!("Reconnection attempt {}/{} failed: {}", 
                             attempt, RECONNECT_ATTEMPTS, e);
                    
                    if attempt < RECONNECT_ATTEMPTS {
                        // Exponential backoff
                        let backoff = Duration::from_millis(500 * 2u64.pow(attempt - 1));
                        std::thread::sleep(backoff);
                    }
                }
            }
        }
        
        Err(std::io::Error::new(
            ErrorKind::ConnectionRefused,
            format!("Failed to reconnect after {} attempts", RECONNECT_ATTEMPTS)
        ))
    }

    pub fn send_move(&mut self, from: (u8, u8), to: (u8, u8), promotion: Option<char>) -> Result<(), std::io::Error> {
        let message = NetworkMessage::Move { from, to, promotion };
        self.send_message(message)
    }
    
    pub fn send_message(&mut self, message: NetworkMessage) -> Result<(), std::io::Error> {
        let serialized = serde_json::to_string(&message)?;
        
        if let Some(stream) = &mut self.stream {
            match stream.write_all(format!("{}\n", serialized).as_bytes()) {
                Ok(_) => {
                    // Update heartbeat timestamp on successful send
                    self.last_heartbeat = Instant::now();
                    Ok(())
                }
                Err(e) => {
                    println!("Error sending message: {}", e);
                    self.stream = None;
                    Err(e)
                }
            }
        } else {
            Err(std::io::Error::new(ErrorKind::NotConnected, "Not connected to server"))
        }
    }

    pub fn receive_message(&mut self) -> Result<Option<NetworkMessage>, std::io::Error> {
        // First, check if we need to send a heartbeat
        if self.is_connected() && self.last_heartbeat.elapsed() > HEARTBEAT_INTERVAL {
            self.send_heartbeat()?;
        }
        
        if self.stream.is_none() {
            return Err(std::io::Error::new(ErrorKind::NotConnected, "Not connected to server"));
        }

        let mut temp_buffer = [0; 1024];
        match self.stream.as_mut().unwrap().read(&mut temp_buffer) {
            Ok(0) => {
                // Connection closed
                println!("Connection closed by server");
                self.stream = None;
                return Err(std::io::Error::new(ErrorKind::ConnectionAborted, "Connection closed"));
            }
            Ok(n) => {
                self.buffer.extend_from_slice(&temp_buffer[..n]);
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                // No data available, continue
            }
            Err(e) => {
                println!("Error reading from server: {}", e);
                self.stream = None;
                return Err(e);
            }
        }

        // Try to find a complete message (ending with newline)
        if let Some(pos) = self.buffer.iter().position(|&b| b == b'\n') {
            let message_bytes = &self.buffer[..pos];
            let message = serde_json::from_slice::<NetworkMessage>(message_bytes);
            
            // Remove the processed message and newline from the buffer
            self.buffer.drain(..=pos);
            
            match message {
                Ok(msg) => {
                    // Update heartbeat timestamp on successful receive
                    if let NetworkMessage::Heartbeat = msg {
                        self.last_heartbeat = Instant::now();
                        return self.receive_message(); // Skip heartbeat messages, try to get real message
                    }
                    Ok(Some(msg))
                }
                Err(e) => {
                    println!("Failed to parse message: {}", e);
                    Err(std::io::Error::new(
                        ErrorKind::InvalidData,
                        format!("Failed to parse message: {}", e)
                    ))
                }
            }
        } else {
            // No complete message yet
            Ok(None)
        }
    }
    
    fn send_heartbeat(&mut self) -> Result<(), std::io::Error> {
        let heartbeat = NetworkMessage::Heartbeat;
        self.send_message(heartbeat)
    }

    pub fn is_white(&self) -> bool {
        matches!(self.role, ClientRole::Player { is_white: true })
    }

    pub fn is_spectator(&self) -> bool {
        matches!(self.role, ClientRole::Spectator)
    }

    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }
    
    pub fn set_role(&mut self, role: ClientRole) {
        self.role = role;
    }
    
    // Draw, resignation, and rematch functionality
    pub fn offer_draw(&mut self) -> Result<(), std::io::Error> {
        let message = NetworkMessage::OfferDraw;
        self.send_message(message)
    }
    
    pub fn accept_draw(&mut self) -> Result<(), std::io::Error> {
        let message = NetworkMessage::AcceptDraw;
        self.send_message(message)
    }
    
    pub fn decline_draw(&mut self) -> Result<(), std::io::Error> {
        let message = NetworkMessage::DeclineDraw;
        self.send_message(message)
    }
    
    pub fn resign(&mut self) -> Result<(), std::io::Error> {
        let message = NetworkMessage::Resign;
        self.send_message(message)
    }
    
    pub fn request_rematch(&mut self) -> Result<(), std::io::Error> {
        let message = NetworkMessage::RequestRematch;
        self.send_message(message)
    }
    
    // New spectator functionality
    pub fn spectate_game(&mut self, game_id: String, spectator_name: String) -> Result<(), std::io::Error> {
        let message = NetworkMessage::SpectateGame { 
            game_id, 
            spectator_name 
        };
        self.send_message(message)
    }
    
    pub fn send_chat_message(&mut self, message: String, name: String) -> Result<(), std::io::Error> {
        let chat_message = NetworkMessage::ChatMessage {
            sender: name,
            message,
            is_spectator: self.is_spectator(),
        };
        self.send_message(chat_message)
    }
}

pub struct ChessServer {
    listener: TcpListener,
}

impl ChessServer {
    pub fn new(port: u16) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
        listener.set_nonblocking(true)?;
        Ok(Self { listener })
    }

    pub fn accept_connections(&self) -> Result<(ChessClient, ChessClient), std::io::Error> {
        println!("Waiting for players to connect...");
        
        // Accept first player
        let (stream1, _) = self.listener.accept()?;
        println!("First player connected");
        
        // Accept second player
        let (stream2, _) = self.listener.accept()?;
        println!("Second player connected");

        // Create clients and assign colors
        let mut client1 = ChessClient {
            stream: Some(stream1),
            is_white: true,
            buffer: Vec::new(),
            server_address: "".to_string(),
            player_name: String::new(),
        };
        let mut client2 = ChessClient {
            stream: Some(stream2),
            is_white: false,
            buffer: Vec::new(),
            server_address: "".to_string(),
            player_name: String::new(),
        };

        // Send color assignments
        let message1 = NetworkMessage::GameStart { is_white: true, game_id: "".to_string(), opponent_name: "".to_string() };
        let message2 = NetworkMessage::GameStart { is_white: false, game_id: "".to_string(), opponent_name: "".to_string() };
        
        client1.stream.as_mut().unwrap().write_all(serde_json::to_string(&message1)?.as_bytes())?;
        client2.stream.as_mut().unwrap().write_all(serde_json::to_string(&message2)?.as_bytes())?;

        Ok((client1, client2))
    }
} 
