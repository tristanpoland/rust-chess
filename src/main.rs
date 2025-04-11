use ggez::{Context, ContextBuilder, GameResult};
use ggez::event::{self, EventHandler};
use ggez::input::mouse::MouseButton;
use ggez::conf::{WindowSetup, WindowMode};
use std::env;
use std::thread;
use std::io::{self, Write};

use chess::gui::ChessGui;
use chess::network::{ChessClient, NetworkMessage, GameInfo};

enum ClientMode {
    Local,
    NetworkHost,
    NetworkJoin(String),
    Observer(String),
}

struct ChessGame {
    gui: ChessGui,
    network_client: Option<ChessClient>,
    game_id: Option<String>,
    player_name: String,
    client_mode: ClientMode,
    available_games: Vec<GameInfo>,
}

impl ChessGame {
    fn new(ctx: &mut Context, client_mode: ClientMode, server_address: Option<&str>, player_name: String) -> GameResult<Self> {
        let gui = ChessGui::new(ctx)?;
        let network_client = if let ClientMode::Local = client_mode {
            None
        } else {
            let client = ChessClient::new(server_address.unwrap_or("localhost:8080"))?;
            Some(client)
        };
        
        Ok(Self { 
            gui,
            network_client,
            game_id: None,
            player_name,
            client_mode,
            available_games: Vec::new(),
        })
    }

    fn setup_network_game(&mut self) -> GameResult<()> {
        if let Some(client) = &mut self.network_client {
            match &self.client_mode {
                ClientMode::NetworkHost => {
                    // Create a new game
                    let create_game = NetworkMessage::CreateGame { 
                        player_name: self.player_name.clone() 
                    };
                    let serialized = serde_json::to_string(&create_game).unwrap();
                    if let Some(stream) = &mut client.stream {
                        stream.write_all(format!("{}\n", serialized).as_bytes())?;
                    }
                    println!("Waiting for another player to join...");
                }
                ClientMode::NetworkJoin(game_id) => {
                    // Join existing game
                    let join_game = NetworkMessage::JoinGame { 
                        game_id: game_id.clone(),
                        player_name: self.player_name.clone() 
                    };
                    let serialized = serde_json::to_string(&join_game).unwrap();
                    if let Some(stream) = &mut client.stream {
                        stream.write_all(format!("{}\n", serialized).as_bytes())?;
                    }
                    println!("Joining game {}...", game_id);
                }
                ClientMode::Observer(game_id) => {
                    // TODO: Implement observer mode
                    println!("Observer mode not implemented yet");
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_network_message(&mut self) -> GameResult<()> {
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
                    self.gui.handle_network_move(from, to, promotion)?;
                }
                Ok(Some(NetworkMessage::GameStart { is_white, game_id })) => {
                    self.gui.set_player_color(is_white);
                    self.game_id = Some(game_id.clone());
                    println!("Game started! You are playing as {}", if is_white { "white" } else { "black" });
                }
                Ok(Some(NetworkMessage::GameState { board, current_turn, promotion_pending, game_over })) => {
                    self.gui.update_game_state(board, current_turn, promotion_pending, game_over)?;
                }
                Ok(Some(NetworkMessage::GameEnd { reason })) => {
                    println!("Game ended: {}", reason);
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
                        println!("{}. {} (hosted by {})", i + 1, game.game_id, game.host_name);
                    }
                }
                Ok(Some(NetworkMessage::CreateGame { .. })) => {
                    // Ignore unexpected CreateGame messages
                    println!("Received unexpected CreateGame message");
                }
                Ok(Some(NetworkMessage::JoinGame { .. })) => {
                    // Ignore unexpected JoinGame messages
                    println!("Received unexpected JoinGame message");
                }
                Ok(Some(NetworkMessage::RequestGameList)) => {
                    // Ignore unexpected RequestGameList messages
                    println!("Received unexpected RequestGameList message");
                }
                Ok(None) => {
                    // No message received, continue
                }
                Err(e) => {
                    println!("Network error: {}", e);
                    if !client.is_connected() {
                        println!("Connection lost, will attempt to reconnect");
                    }
                }
            }
        }
        Ok(())
    }
}

impl EventHandler for ChessGame {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        self.handle_network_message()?;
        self.gui.update()
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        self.gui.draw(ctx)
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult<()> {
        if let Some(move_info) = self.gui.handle_mouse_down(button, x, y)? {
            if let Some(client) = &mut self.network_client {
                if !client.is_connected() {
                    println!("Cannot send move - not connected to server");
                    return Ok(());
                }
                if let Err(e) = client.send_move(move_info.from, move_info.to, move_info.promotion) {
                    println!("Error sending move: {}", e);
                }
            }
        }
        Ok(())
    }
    
    fn mouse_motion_event(
        &mut self,
        _ctx: &mut Context,
        x: f32,
        y: f32,
        _dx: f32,
        _dy: f32,
    ) -> GameResult<()> {
        self.gui.handle_mouse_move(x, y)
    }
}

fn main() -> GameResult {
    println!("Starting Rust Chess Game");
    
    let args: Vec<String> = env::args().collect();
    let is_server = args.iter().any(|arg| arg == "--server");
    let is_network = args.iter().any(|arg| arg == "--network");
    let server_address = args.iter().position(|arg| arg == "--address")
        .and_then(|pos| args.get(pos + 1))
        .map(|s| s.as_str());
    let join_game = args.iter().position(|arg| arg == "--join")
        .and_then(|pos| args.get(pos + 1))
        .map(|s| s.to_string());
    
    if is_server {
        println!("Starting server mode...");
        let mut server = chess::server::ChessServer::new(8080)?;
        server.run()?;
        Ok(())
    } else {
        let player_name = args.iter().position(|arg| arg == "--name")
            .and_then(|pos| args.get(pos + 1))
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                print!("Enter your player name: ");
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                input.trim().to_string()
            });
            
        let client_mode = if is_network {
            if let Some(game_id) = join_game {
                ClientMode::NetworkJoin(game_id)
            } else {
                // Show available games or create a new one
                print!("Do you want to (1) Create a new game or (2) Join an existing game? ");
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                
                match input.trim() {
                    "1" => ClientMode::NetworkHost,
                    "2" => {
                        // Create a temporary client to get game list
                        let mut temp_client = ChessClient::new(server_address.unwrap_or("localhost:8080"))?;
                        let request = NetworkMessage::RequestGameList;
                        if let Some(stream) = &mut temp_client.stream {
                            let serialized = serde_json::to_string(&request).unwrap();
                            stream.write_all(format!("{}\n", serialized).as_bytes())?;
                        }
                        
                        // Wait briefly for response
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        
                        // Process game list
                        let mut games = Vec::new();
                        match temp_client.receive_message() {
                            Ok(Some(NetworkMessage::GameList { available_games })) => {
                                games = available_games;
                                println!("Available games:");
                                for (i, game) in games.iter().enumerate() {
                                    println!("{}. {} (hosted by {})", i + 1, game.game_id, game.host_name);
                                }
                            },
                            _ => {
                                println!("No games available or error fetching game list");
                            }
                        }
                        
                        if games.is_empty() {
                            println!("No games available. Creating a new game instead.");
                            ClientMode::NetworkHost
                        } else {
                            print!("Enter game number to join: ");
                            io::stdout().flush().unwrap();
                            let mut input = String::new();
                            io::stdin().read_line(&mut input).unwrap();
                            
                            match input.trim().parse::<usize>() {
                                Ok(num) if num > 0 && num <= games.len() => {
                                    ClientMode::NetworkJoin(games[num-1].game_id.clone())
                                },
                                _ => {
                                    println!("Invalid selection. Creating a new game instead.");
                                    ClientMode::NetworkHost
                                }
                            }
                        }
                    },
                    _ => {
                        println!("Invalid choice. Creating a new game.");
                        ClientMode::NetworkHost
                    }
                }
            }
        } else {
            ClientMode::Local
        };

        let resource_dir = std::path::PathBuf::from("./assets");
        
        let (mut ctx, event_loop) = ContextBuilder::new("chess", "Rust Chess")
            .window_setup(WindowSetup::default().title("Rust Chess"))
            .window_mode(WindowMode::default().dimensions(600.0, 750.0))
            .add_resource_path(resource_dir)
            .build()?;

        let mut game = ChessGame::new(&mut ctx, client_mode, server_address, player_name)?;
        
        // Set up network connection if needed
        if is_network {
            game.setup_network_game()?;
        }
        
        println!("Game initialized, running event loop");
        event::run(ctx, event_loop, game)
    }
}
