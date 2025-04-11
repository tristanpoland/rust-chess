use ggez::{Context, ContextBuilder, GameResult};
use ggez::event::{self, EventHandler};
use ggez::input::mouse::MouseButton;
use ggez::conf::{WindowSetup, WindowMode};
use std::env;
use std::thread;

use chess::gui::ChessGui;
use chess::network::{ChessClient, NetworkMessage};

struct ChessGame {
    gui: ChessGui,
    network_client: Option<ChessClient>,
}

impl ChessGame {
    fn new(ctx: &mut Context, is_network: bool, server_address: Option<&str>) -> GameResult<Self> {
        let gui = ChessGui::new(ctx)?;
        let network_client = if is_network {
            let client = ChessClient::new(server_address.unwrap_or("localhost:8080"))?;
            Some(client)
        } else {
            None
        };
        Ok(Self { gui, network_client })
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
                Ok(Some(NetworkMessage::GameStart { is_white })) => {
                    self.gui.set_player_color(is_white);
                }
                Ok(Some(NetworkMessage::GameState { board, current_turn, promotion_pending, game_over })) => {
                    self.gui.update_game_state(board, current_turn, promotion_pending, game_over)?;
                }
                Ok(Some(NetworkMessage::GameEnd { reason })) => {
                    println!("Game ended: {}", reason);
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

    if is_server {
        println!("Starting server mode...");
        let mut server = chess::server::ChessServer::new(8080)?;
        server.run()?;
        Ok(())
    } else {
        let resource_dir = std::path::PathBuf::from("./assets");
        
        let (mut ctx, event_loop) = ContextBuilder::new("chess", "Rust Chess")
            .window_setup(WindowSetup::default().title("Rust Chess"))
            .window_mode(WindowMode::default().dimensions(600.0, 750.0))
            .add_resource_path(resource_dir)
            .build()?;

        let game = ChessGame::new(&mut ctx, is_network, server_address)?;
        
        println!("Game initialized, running event loop");
        event::run(ctx, event_loop, game)
    }
}
