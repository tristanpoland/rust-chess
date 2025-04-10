use ggez::{Context, ContextBuilder, GameResult};
use ggez::event::{self, EventHandler};
use ggez::input::mouse::MouseButton;
use ggez::conf::{WindowSetup, WindowMode};

use chess::gui::ChessGui;

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
        self.gui.handle_mouse_down(button, x, y)
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
    
    let resource_dir = std::path::PathBuf::from("./assets");
    
    let (mut ctx, event_loop) = ContextBuilder::new("chess", "Rust Chess")
        .window_setup(WindowSetup::default().title("Rust Chess"))
        .window_mode(WindowMode::default().dimensions(600.0, 750.0))
        .add_resource_path(resource_dir)
        .build()?;

    let game = ChessGame::new(&mut ctx)?;
    
    println!("Game initialized, running event loop");
    event::run(ctx, event_loop, game)
}
