pub mod doom;
pub mod snake;

pub use doom::DoomGame;
pub use snake::{GameState, SnakeGame};

use image::RgbImage;
use rusttype::Font;

/// Active game variant — wraps all supported games with a unified interface
pub enum ActiveGame {
    Snake(SnakeGame),
    Doom(DoomGame),
}

impl ActiveGame {
    pub fn tick(&mut self) {
        match self {
            ActiveGame::Snake(g) => g.tick(),
            ActiveGame::Doom(g) => g.tick(),
        }
    }

    pub fn render_button(&self, button_id: u8) -> RgbImage {
        match self {
            ActiveGame::Snake(g) => g.render_button(button_id),
            ActiveGame::Doom(g) => g.render_button(button_id),
        }
    }

    pub fn render_strip(&self, font: &Font) -> RgbImage {
        match self {
            ActiveGame::Snake(g) => g.render_strip(font),
            ActiveGame::Doom(g) => g.render_strip(font),
        }
    }

    pub fn handle_encoder_rotate(&mut self, encoder: u8, direction: i8) {
        match self {
            ActiveGame::Snake(g) => g.handle_encoder_rotate(encoder, direction),
            ActiveGame::Doom(g) => g.handle_encoder_rotate(encoder, direction),
        }
    }

    pub fn handle_encoder_press(&mut self, encoder: u8) {
        match self {
            ActiveGame::Snake(_) => {} // Snake doesn't use encoder presses
            ActiveGame::Doom(g) => g.handle_encoder_press(encoder),
        }
    }

    pub fn handle_button_press(&mut self, button_id: u8) {
        match self {
            ActiveGame::Snake(g) => g.handle_button_press(button_id),
            ActiveGame::Doom(g) => g.handle_button_press(button_id),
        }
    }

    pub fn is_button_dirty(&self, button_id: u8) -> bool {
        match self {
            ActiveGame::Snake(g) => g.is_button_dirty(button_id),
            ActiveGame::Doom(g) => g.is_button_dirty(button_id),
        }
    }

    pub fn is_strip_dirty(&self) -> bool {
        match self {
            ActiveGame::Snake(g) => g.is_strip_dirty(),
            ActiveGame::Doom(g) => g.is_strip_dirty(),
        }
    }

    pub fn has_any_dirty(&self) -> bool {
        match self {
            ActiveGame::Snake(g) => g.has_any_dirty(),
            ActiveGame::Doom(g) => g.has_any_dirty(),
        }
    }

    pub fn clear_dirty(&mut self) {
        match self {
            ActiveGame::Snake(g) => g.clear_dirty(),
            ActiveGame::Doom(g) => g.clear_dirty(),
        }
    }
}
