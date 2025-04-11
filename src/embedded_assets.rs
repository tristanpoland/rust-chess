use ggez::graphics::Image;
use ggez::{Context, GameResult};
use std::collections::HashMap;
use std::io::Cursor;
use ggez::graphics::DrawParam;

use crate::piece::{PieceType, Color};

pub struct EmbeddedAssets {
    piece_images: HashMap<(PieceType, Color), Image>,
}

impl EmbeddedAssets {
    pub fn new(ctx: &mut Context) -> GameResult<Self> {
        let mut piece_images = HashMap::new();
        
        // White pieces
        Self::load_piece_image(ctx, &mut piece_images, PieceType::King, Color::White, 
            include_bytes!("../embedded_assets/white_king.png"))?;
        Self::load_piece_image(ctx, &mut piece_images, PieceType::Queen, Color::White, 
            include_bytes!("../embedded_assets/white_queen.png"))?;
        Self::load_piece_image(ctx, &mut piece_images, PieceType::Rook, Color::White, 
            include_bytes!("../embedded_assets/white_rook.png"))?;
        Self::load_piece_image(ctx, &mut piece_images, PieceType::Bishop, Color::White, 
            include_bytes!("../embedded_assets/white_bishop.png"))?;
        Self::load_piece_image(ctx, &mut piece_images, PieceType::Knight, Color::White, 
            include_bytes!("../embedded_assets/white_knight.png"))?;
        Self::load_piece_image(ctx, &mut piece_images, PieceType::Pawn, Color::White, 
            include_bytes!("../embedded_assets/white_pawn.png"))?;
        
        // Black pieces
        Self::load_piece_image(ctx, &mut piece_images, PieceType::King, Color::Black, 
            include_bytes!("../embedded_assets/black_king.png"))?;
        Self::load_piece_image(ctx, &mut piece_images, PieceType::Queen, Color::Black, 
            include_bytes!("../embedded_assets/black_queen.png"))?;
        Self::load_piece_image(ctx, &mut piece_images, PieceType::Rook, Color::Black, 
            include_bytes!("../embedded_assets/black_rook.png"))?;
        Self::load_piece_image(ctx, &mut piece_images, PieceType::Bishop, Color::Black, 
            include_bytes!("../embedded_assets/black_bishop.png"))?;
        Self::load_piece_image(ctx, &mut piece_images, PieceType::Knight, Color::Black, 
            include_bytes!("../embedded_assets/black_knight.png"))?;
        Self::load_piece_image(ctx, &mut piece_images, PieceType::Pawn, Color::Black, 
            include_bytes!("../embedded_assets/black_pawn.png"))?;
        
        Ok(Self {
            piece_images,
        })
    }
    
    fn load_piece_image(
        ctx: &mut Context,
        piece_images: &mut HashMap<(PieceType, Color), Image>,
        piece_type: PieceType,
        color: Color,
        image_data: &'static [u8],
    ) -> GameResult<()> {
        let reader = Cursor::new(image_data);
        let image = Image::from_bytes(ctx, reader.into_inner())?;
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