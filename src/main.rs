use ggez::{Context, ContextBuilder, GameResult};
use ggez::event::{self, EventHandler};
use ggez::input::mouse::MouseButton;
use ggez::conf::{WindowSetup, WindowMode};
use std::env;
use std::thread;
use std::io::{self, Write};

use chess::gui::ChessGui;

enum GameMode {
    Local,
    NetworkHost,
    NetworkJoin(String),
    Observer(String),
}

struct ChessGame {
    gui: ChessGui,
}

impl ChessGame {
    fn new(ctx: &mut Context) -> GameResult<Self> {
        let gui = ChessGui::new(ctx)?;
        Ok(Self { gui })
    }
}

impl EventHandler for ChessGame {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
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
        self.gui.handle_mouse_down(button, x, y)?;
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
        .map(|s| s.as_str())
        .unwrap_or("localhost:8080");
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
            
        let game_mode = if is_network {
            if let Some(game_id) = join_game {
                GameMode::NetworkJoin(game_id)
            } else {
                // Ask the user whether to create or join a game
                print!("Do you want to (1) Create a new game or (2) Join an existing game? ");
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                
                match input.trim() {
                    "1" => GameMode::NetworkHost,
                    "2" => GameMode::NetworkJoin(String::new()), // Will query available games later
                    _ => {
                        println!("Invalid choice. Creating a new game.");
                        GameMode::NetworkHost
                    }
                }
            }
        } else {
            GameMode::Local
        };

        let resource_dir = std::path::PathBuf::from("./assets");
        
        let (mut ctx, event_loop) = ContextBuilder::new("chess", "Rust Chess")
            .window_setup(WindowSetup::default().title("Rust Chess"))
            .window_mode(WindowMode::default().dimensions(780.0, 750.0))
            .add_resource_path(resource_dir)
            .build()?;

        let mut game = ChessGame::new(&mut ctx)?;
        
        // Set up network connection if needed
        if is_network {
            // Set server address
            game.gui.set_server_address(server_address.to_string());
            
            // Initialize network connection
            game.gui.init_network(server_address, player_name)?;
            
            match game_mode {
                GameMode::NetworkHost => {
                    // Create a new game
                    game.gui.create_game()?;
                },
                GameMode::NetworkJoin(game_id) if !game_id.is_empty() => {
                    // Join a specific game
                    game.gui.join_game(game_id)?;
                },
                GameMode::NetworkJoin(_) => {
                    // Get list of available games and let user choose
                    game.gui.request_game_list()?;
                    
                    // Wait a moment for the server to respond
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    
                    // Update GUI to process messages (get game list)
                    game.gui.update()?;
                    
                    let available_games = game.gui.get_available_games();
                    
                    if available_games.is_empty() {
                        println!("No games available. Creating a new game instead.");
                        game.gui.create_game()?;
                    } else {
                        println!("Available games:");
                        for (i, game_info) in available_games.iter().enumerate() {
                            println!("{}. {} (hosted by {})", i + 1, game_info.game_id, game_info.host_name);
                        }
                        
                        print!("Enter game number to join: ");
                        io::stdout().flush().unwrap();
                        let mut input = String::new();
                        io::stdin().read_line(&mut input).unwrap();
                        
                        match input.trim().parse::<usize>() {
                            Ok(num) if num > 0 && num <= available_games.len() => {
                                let selected_game = &available_games[num - 1];
                                game.gui.join_game(selected_game.game_id.clone())?;
                            },
                            _ => {
                                println!("Invalid selection. Creating a new game instead.");
                                game.gui.create_game()?;
                            }
                        }
                    }
                },
                GameMode::Observer(_) => {
                    // Not implemented yet
                    println!("Observer mode not implemented yet");
                },
                _ => {}
            }
        }
        
        println!("Game initialized, running event loop");
        event::run(ctx, event_loop, game)
    }
}
