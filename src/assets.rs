use ggez::{Context, GameResult};
use ggez::graphics::{Image, DrawParam};
use std::collections::HashMap;
use std::path::Path;

use crate::piece::{PieceType, Color};

pub struct Assets {
    piece_images: HashMap<(PieceType, Color), Image>,
}

impl Assets {
    pub fn new(ctx: &mut Context) -> GameResult<Self> {
        let mut piece_images = HashMap::new();
        
        Self::load_piece_image(ctx, &mut piece_images, PieceType::King, Color::White, "/images/white_king.png")?;
        Self::load_piece_image(ctx, &mut piece_images, PieceType::Queen, Color::White, "/images/white_queen.png")?;
        Self::load_piece_image(ctx, &mut piece_images, PieceType::Rook, Color::White, "/images/white_rook.png")?;
        Self::load_piece_image(ctx, &mut piece_images, PieceType::Bishop, Color::White, "/images/white_bishop.png")?;
        Self::load_piece_image(ctx, &mut piece_images, PieceType::Knight, Color::White, "/images/white_knight.png")?;
        Self::load_piece_image(ctx, &mut piece_images, PieceType::Pawn, Color::White, "/images/white_pawn.png")?;
        
        Self::load_piece_image(ctx, &mut piece_images, PieceType::King, Color::Black, "/images/black_king.png")?;
        Self::load_piece_image(ctx, &mut piece_images, PieceType::Queen, Color::Black, "/images/black_queen.png")?;
        Self::load_piece_image(ctx, &mut piece_images, PieceType::Rook, Color::Black, "/images/black_rook.png")?;
        Self::load_piece_image(ctx, &mut piece_images, PieceType::Bishop, Color::Black, "/images/black_bishop.png")?;
        Self::load_piece_image(ctx, &mut piece_images, PieceType::Knight, Color::Black, "/images/black_knight.png")?;
        Self::load_piece_image(ctx, &mut piece_images, PieceType::Pawn, Color::Black, "/images/black_pawn.png")?;
        
        Ok(Self {
            piece_images,
        })
    }
    
    fn load_piece_image(
        ctx: &mut Context,
        piece_images: &mut HashMap<(PieceType, Color), Image>,
        piece_type: PieceType,
        color: Color,
        path: &str,
    ) -> GameResult<()> {
        let image = Image::from_path(ctx, Path::new(path))?;
        piece_images.insert((piece_type, color), image);
        Ok(())
    }
    
    pub fn get_piece_image(&self, piece_type: PieceType, color: Color) -> &Image {
        self.piece_images.get(&(piece_type, color)).expect("Missing piece image")
    }
    
    pub fn draw_piece(
        &self,
        canvas: &mut ggez::graphics::Canvas,
        piece_type: PieceType,
        color: Color,
        param: DrawParam,
    ) {
        let image = self.get_piece_image(piece_type, color);
        canvas.draw(image, param);
    }
} 
