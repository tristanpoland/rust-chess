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
}

pub struct ChessClient {
    pub stream: TcpStream,
    pub is_white: bool,
    buffer: Vec<u8>,
}

impl ChessClient {
    pub fn new(server_address: &str) -> Result<Self, std::io::Error> {
        let mut stream = TcpStream::connect(server_address)?;
        stream.set_nonblocking(true)?;
        Ok(Self {
            stream,
            is_white: false,
            buffer: Vec::new(),
        })
    }

    pub fn with_color(stream: TcpStream, is_white: bool) -> Self {
        Self {
            stream,
            is_white,
            buffer: Vec::new(),
        }
    }

    pub fn send_move(&mut self, from: (u8, u8), to: (u8, u8), promotion: Option<char>) -> Result<(), std::io::Error> {
        let message = NetworkMessage::Move { from, to, promotion };
        let serialized = serde_json::to_string(&message)?;
        self.stream.write_all(serialized.as_bytes())?;
        Ok(())
    }

    pub fn receive_message(&mut self) -> Result<Option<NetworkMessage>, std::io::Error> {
        let mut temp_buffer = [0; 1024];
        match self.stream.read(&mut temp_buffer) {
            Ok(0) => {
                // Connection closed
                return Err(std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "Connection closed"));
            }
            Ok(n) => {
                self.buffer.extend_from_slice(&temp_buffer[..n]);
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                // No data available, continue
            }
            Err(e) => return Err(e),
        }

        // Try to parse a complete message
        match serde_json::from_slice::<NetworkMessage>(&self.buffer) {
            Ok(message) => {
                // Find the end of the JSON message
                if let Some(pos) = self.buffer.iter().position(|&b| b == b'}') {
                    // Remove the processed message from the buffer
                    self.buffer.drain(..=pos);
                    Ok(Some(message))
                } else {
                    // Incomplete message, keep waiting
                    Ok(None)
                }
            }
            Err(e) if e.is_eof() => {
                // Incomplete message, keep waiting
                Ok(None)
            }
            Err(e) => {
                // Invalid message, clear buffer and return error
                self.buffer.clear();
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to parse message: {}", e)
                ))
            }
        }
    }

    pub fn is_white(&self) -> bool {
        self.is_white
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
            stream: stream1,
            is_white: true,
            buffer: Vec::new(),
        };
        let mut client2 = ChessClient {
            stream: stream2,
            is_white: false,
            buffer: Vec::new(),
        };

        // Send color assignments
        let message1 = NetworkMessage::GameStart { is_white: true };
        let message2 = NetworkMessage::GameStart { is_white: false };
        
        client1.stream.write_all(serde_json::to_string(&message1)?.as_bytes())?;
        client2.stream.write_all(serde_json::to_string(&message2)?.as_bytes())?;

        Ok((client1, client2))
    }
} 