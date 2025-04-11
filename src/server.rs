use std::net::TcpListener;
use std::io::Write;
use serde_json;
use crate::network::{ChessClient, NetworkMessage};
use crate::board::GameState;
use crate::piece::{Color, PieceType};

pub struct ChessServer {
    listener: TcpListener,
    game_state: GameState,
}

impl ChessServer {
    pub fn new(port: u16) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
        Ok(Self { 
            listener,
            game_state: GameState::new(),
        })
    }

    fn broadcast_game_state(&self, client1: &mut ChessClient, client2: &mut ChessClient) -> Result<(), std::io::Error> {
        let board_state = self.game_state.board.map(|row| {
            row.map(|cell| cell.map(|piece| (piece.piece_type, piece.color)))
        });

        let message = NetworkMessage::GameState {
            board: board_state,
            current_turn: self.game_state.current_turn,
            promotion_pending: self.game_state.promotion_pending.as_ref().map(|p| (p.position.0, p.position.1, p.color)),
            game_over: self.game_state.is_game_over(),
        };

        let serialized = serde_json::to_string(&message)?;
        client1.stream.write_all(serialized.as_bytes())?;
        client2.stream.write_all(serialized.as_bytes())?;
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), std::io::Error> {
        println!("Waiting for players to connect...");
        
        // Accept first player
        let (mut stream1, _) = self.listener.accept()?;
        stream1.set_nonblocking(true)?;
        println!("First player connected");
        
        // Accept second player
        let (mut stream2, _) = self.listener.accept()?;
        stream2.set_nonblocking(true)?;
        println!("Second player connected");

        // Create clients and assign colors
        let mut client1 = ChessClient::with_color(stream1, true);
        let mut client2 = ChessClient::with_color(stream2, false);

        // Send color assignments
        let message1 = NetworkMessage::GameStart { is_white: true };
        let message2 = NetworkMessage::GameStart { is_white: false };
        
        client1.stream.write_all(serde_json::to_string(&message1)?.as_bytes())?;
        client2.stream.write_all(serde_json::to_string(&message2)?.as_bytes())?;

        // Send initial game state
        self.broadcast_game_state(&mut client1, &mut client2)?;

        // Start game loop
        let mut current_turn = true; // true for white, false for black
        
        loop {
            let (sender, receiver) = if current_turn {
                (&mut client1, &mut client2)
            } else {
                (&mut client2, &mut client1)
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
                                _ => continue,
                            };
                            self.game_state.promote_pawn(piece_type);
                        }

                        // Switch turns
                        current_turn = !current_turn;

                        // Broadcast updated game state to both clients
                        self.broadcast_game_state(&mut client1, &mut client2)?;
                    }
                }
                Ok(Some(NetworkMessage::GameStart { .. })) => {
                    // Ignore GameStart messages after initial setup
                }
                Ok(Some(NetworkMessage::GameState { .. })) => {
                    // Ignore GameState messages from clients
                }
                Ok(Some(NetworkMessage::GameEnd { reason })) => {
                    // Forward game end to both players
                    let end_message = NetworkMessage::GameEnd { reason: reason.clone() };
                    client1.stream.write_all(serde_json::to_string(&end_message)?.as_bytes())?;
                    client2.stream.write_all(serde_json::to_string(&end_message)?.as_bytes())?;
                    break;
                }
                Ok(None) => {
                    // No message received, continue
                }
                Err(e) => {
                    println!("Error receiving message: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }
} 