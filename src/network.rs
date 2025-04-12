use std::net::{TcpStream, TcpListener};
use std::io::{Read, Write};
use serde::{Serialize, Deserialize};
use std::io::ErrorKind;
use crate::piece::{PieceType, Color};

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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameInfo {
    pub game_id: String,
    pub host_name: String,
    pub status: GameStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum GameStatus {
    Waiting,
    InProgress,
    Completed,
}

pub struct ChessClient {
    pub stream: Option<TcpStream>,
    pub is_white: bool,
    buffer: Vec<u8>,
    server_address: String,
    pub player_name: String,
}

impl ChessClient {
    pub fn new(server_address: &str) -> Result<Self, std::io::Error> {
        let stream = TcpStream::connect(server_address)?;
        stream.set_nonblocking(true)?;
        Ok(Self {
            stream: Some(stream),
            is_white: false,
            buffer: Vec::new(),
            server_address: server_address.to_string(),
            player_name: String::new(),
        })
    }

    pub fn with_color(stream: TcpStream, is_white: bool, server_address: &str) -> Self {
        Self {
            stream: Some(stream),
            is_white,
            buffer: Vec::new(),
            server_address: server_address.to_string(),
            player_name: String::new(),
        }
    }

    pub fn reconnect(&mut self) -> Result<(), std::io::Error> {
        println!("Attempting to reconnect to server...");
        match TcpStream::connect(&self.server_address) {
            Ok(stream) => {
                stream.set_nonblocking(true)?;
                self.stream = Some(stream);
                println!("Successfully reconnected to server");
                Ok(())
            }
            Err(e) => {
                println!("Failed to reconnect: {}", e);
                Err(e)
            }
        }
    }

    pub fn send_move(&mut self, from: (u8, u8), to: (u8, u8), promotion: Option<char>) -> Result<(), std::io::Error> {
        let message = NetworkMessage::Move { from, to, promotion };
        let serialized = serde_json::to_string(&message)?;
        
        if let Some(stream) = &mut self.stream {
            match stream.write_all(format!("{}\n", serialized).as_bytes()) {
                Ok(_) => Ok(()),
                Err(e) => {
                    println!("Error sending move: {}", e);
                    self.stream = None;
                    Err(e)
                }
            }
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected to server"))
        }
    }

    pub fn receive_message(&mut self) -> Result<Option<NetworkMessage>, std::io::Error> {
        if self.stream.is_none() {
            return Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected to server"));
        }

        let mut temp_buffer = [0; 1024];
        match self.stream.as_mut().unwrap().read(&mut temp_buffer) {
            Ok(0) => {
                // Connection closed
                println!("Connection closed by server");
                self.stream = None;
                return Err(std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "Connection closed"));
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
                Ok(msg) => Ok(Some(msg)),
                Err(e) => {
                    println!("Failed to parse message: {}", e);
                    Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Failed to parse message: {}", e)
                    ))
                }
            }
        } else {
            // No complete message yet
            Ok(None)
        }
    }

    pub fn is_white(&self) -> bool {
        self.is_white
    }

    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }
    
    // New methods for draw, resignation, and rematch functionality
    pub fn offer_draw(&mut self) -> Result<(), std::io::Error> {
        let message = NetworkMessage::OfferDraw;
        let serialized = serde_json::to_string(&message)?;
        
        if let Some(stream) = &mut self.stream {
            match stream.write_all(format!("{}\n", serialized).as_bytes()) {
                Ok(_) => Ok(()),
                Err(e) => {
                    println!("Error offering draw: {}", e);
                    self.stream = None;
                    Err(e)
                }
            }
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected to server"))
        }
    }
    
    pub fn accept_draw(&mut self) -> Result<(), std::io::Error> {
        let message = NetworkMessage::AcceptDraw;
        let serialized = serde_json::to_string(&message)?;
        
        if let Some(stream) = &mut self.stream {
            match stream.write_all(format!("{}\n", serialized).as_bytes()) {
                Ok(_) => Ok(()),
                Err(e) => {
                    println!("Error accepting draw: {}", e);
                    self.stream = None;
                    Err(e)
                }
            }
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected to server"))
        }
    }
    
    pub fn decline_draw(&mut self) -> Result<(), std::io::Error> {
        let message = NetworkMessage::DeclineDraw;
        let serialized = serde_json::to_string(&message)?;
        
        if let Some(stream) = &mut self.stream {
            match stream.write_all(format!("{}\n", serialized).as_bytes()) {
                Ok(_) => Ok(()),
                Err(e) => {
                    println!("Error declining draw: {}", e);
                    self.stream = None;
                    Err(e)
                }
            }
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected to server"))
        }
    }
    
    pub fn resign(&mut self) -> Result<(), std::io::Error> {
        let message = NetworkMessage::Resign;
        let serialized = serde_json::to_string(&message)?;
        
        if let Some(stream) = &mut self.stream {
            match stream.write_all(format!("{}\n", serialized).as_bytes()) {
                Ok(_) => Ok(()),
                Err(e) => {
                    println!("Error resigning: {}", e);
                    self.stream = None;
                    Err(e)
                }
            }
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected to server"))
        }
    }
    
    pub fn request_rematch(&mut self) -> Result<(), std::io::Error> {
        let message = NetworkMessage::RequestRematch;
        let serialized = serde_json::to_string(&message)?;
        
        if let Some(stream) = &mut self.stream {
            match stream.write_all(format!("{}\n", serialized).as_bytes()) {
                Ok(_) => Ok(()),
                Err(e) => {
                    println!("Error requesting rematch: {}", e);
                    self.stream = None;
                    Err(e)
                }
            }
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected to server"))
        }
    }
}

pub struct ChessServer {
    listener: TcpListener,
}

impl ChessServer {
    pub fn new(port: u16) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
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