use anyhow::Result;
use image::{Rgb, RgbImage};
use rusttype::{Font, Scale};
use std::collections::HashMap;
use tracing::debug;

use crate::config::Config;
use crate::device::BUTTON_LABELS;
use crate::state::AppState;

use super::buttons::render_button_image;
use super::strip::render_strip_image;

/// Color constants
pub const WHITE: Rgb<u8> = Rgb([255, 255, 255]);
pub const GREEN: Rgb<u8> = Rgb([0, 200, 100]);
pub const BRIGHT_GREEN: Rgb<u8> = Rgb([50, 220, 130]);
pub const RED: Rgb<u8> = Rgb([220, 60, 60]);
pub const BRIGHT_RED: Rgb<u8> = Rgb([255, 80, 80]);
pub const BLUE: Rgb<u8> = Rgb([60, 120, 200]);
pub const BRIGHT_BLUE: Rgb<u8> = Rgb([80, 150, 240]);
pub const PURPLE: Rgb<u8> = Rgb([140, 80, 200]);
pub const BRIGHT_PURPLE: Rgb<u8> = Rgb([170, 100, 240]);
pub const GRAY: Rgb<u8> = Rgb([80, 85, 95]);
pub const BRIGHT_GRAY: Rgb<u8> = Rgb([110, 115, 125]);
pub const ORANGE: Rgb<u8> = Rgb([220, 140, 50]);
pub const DARK_BG: Rgb<u8> = Rgb([15, 15, 22]);
#[allow(dead_code)]
pub const BUTTON_BG: Rgb<u8> = Rgb([25, 28, 38]);
#[allow(dead_code)]
pub const BUTTON_ACTIVE: Rgb<u8> = Rgb([0, 120, 80]);

/// Button color scheme by ID
pub fn button_colors(button_id: u8) -> (Rgb<u8>, Rgb<u8>) {
    match button_id {
        0 => (GREEN, BRIGHT_GREEN),   // ACCEPT - green
        1 => (RED, BRIGHT_RED),       // REJECT - red
        2 => (RED, BRIGHT_RED),       // STOP - red
        3 => (GRAY, BRIGHT_GRAY),     // RETRY - gray
        4 => (BLUE, BRIGHT_BLUE),     // REWIND - blue
        5 => (GREEN, BRIGHT_GREEN),   // YES ALL - green
        6 => (BLUE, BRIGHT_BLUE),     // TAB - blue
        7 => (PURPLE, BRIGHT_PURPLE), // MIC - purple
        8 => (BLUE, BRIGHT_BLUE),     // ENTER - blue
        9 => (GRAY, BRIGHT_GRAY),     // UNDO - gray
        _ => (GRAY, BRIGHT_GRAY),
    }
}

/// Renders images for the device display
pub struct DisplayRenderer {
    font: Font<'static>,
    #[allow(dead_code)]
    config: Config,
    icon_cache: HashMap<String, RgbImage>,
}

impl DisplayRenderer {
    pub fn new(config: &Config) -> Result<Self> {
        // Load embedded font (or fall back to system font)
        let font_data = include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf");
        let font = Font::try_from_bytes(font_data as &[u8])
            .ok_or_else(|| anyhow::anyhow!("Failed to load font"))?;

        Ok(Self {
            font,
            config: config.clone(),
            icon_cache: HashMap::new(),
        })
    }

    /// Render a button image
    pub fn render_button(&self, button_id: u8, active: bool, state: &AppState) -> Result<RgbImage> {
        let label = BUTTON_LABELS.get(button_id as usize).unwrap_or(&"?");

        // MIC button (7) uses special icon rendering
        if button_id == 7 {
            super::buttons::render_mic_button(&self.font, active, state.dictation_active, button_id)
        } else {
            render_button_image(&self.font, label, active, button_id)
        }
    }

    /// Render the LCD strip with current state (legacy - not used for N4)
    pub fn render_strip(&self, state: &AppState) -> Result<RgbImage> {
        render_strip_image(&self.font, state)
    }

    /// Render a single LCD strip soft button
    pub fn render_strip_button(&self, button_id: u8, state: &AppState) -> Result<RgbImage> {
        super::strip::render_strip_button(&self.font, button_id, state)
    }

    /// Render full-width strip and slice into 4 parts for N4
    pub fn render_strip_slices(&self, state: &AppState) -> Result<[RgbImage; 4]> {
        use crate::device::{STRIP_BUTTON_HEIGHT, STRIP_BUTTON_WIDTH, STRIP_HEIGHT, STRIP_WIDTH};
        use image::imageops::{crop_imm, resize, FilterType};

        // Render full 480x128 strip
        let full_strip = render_strip_image(&self.font, state)?;

        // Slice into 4 parts (120x128 each) and resize to 112x112
        let slice_width = STRIP_WIDTH / 4; // 120
        let mut slices: [RgbImage; 4] =
            std::array::from_fn(|_| RgbImage::new(STRIP_BUTTON_WIDTH, STRIP_BUTTON_HEIGHT));

        for (i, slot) in slices.iter_mut().enumerate() {
            let x = i as u32 * slice_width;
            let slice = crop_imm(&full_strip, x, 0, slice_width, STRIP_HEIGHT).to_image();
            *slot = resize(
                &slice,
                STRIP_BUTTON_WIDTH,
                STRIP_BUTTON_HEIGHT,
                FilterType::Lanczos3,
            );
        }

        Ok(slices)
    }

    /// Load and cache an icon
    #[allow(dead_code)]
    pub fn load_icon(&mut self, name: &str) -> Option<&RgbImage> {
        if !self.icon_cache.contains_key(name) {
            let path = format!("assets/icons/{}", name);
            if let Ok(img) = image::open(&path) {
                let rgb = img.to_rgb8();
                self.icon_cache.insert(name.to_string(), rgb);
                debug!("Loaded icon: {}", name);
            }
        }
        self.icon_cache.get(name)
    }
}

/// Draw text onto an image
pub fn draw_text(
    image: &mut RgbImage,
    font: &Font,
    text: &str,
    x: i32,
    y: i32,
    scale: f32,
    color: Rgb<u8>,
) {
    let scale = Scale::uniform(scale);
    let v_metrics = font.v_metrics(scale);
    let offset = rusttype::point(x as f32, y as f32 + v_metrics.ascent);

    for glyph in font.layout(text, scale, offset) {
        if let Some(bb) = glyph.pixel_bounding_box() {
            glyph.draw(|gx, gy, v| {
                let px = bb.min.x + gx as i32;
                let py = bb.min.y + gy as i32;

                if px >= 0 && px < image.width() as i32 && py >= 0 && py < image.height() as i32 {
                    let pixel = image.get_pixel_mut(px as u32, py as u32);
                    // Alpha blend
                    let alpha = v;
                    pixel[0] = ((1.0 - alpha) * pixel[0] as f32 + alpha * color[0] as f32) as u8;
                    pixel[1] = ((1.0 - alpha) * pixel[1] as f32 + alpha * color[1] as f32) as u8;
                    pixel[2] = ((1.0 - alpha) * pixel[2] as f32 + alpha * color[2] as f32) as u8;
                }
            });
        }
    }
}

/// Calculate text width
pub fn text_width(font: &Font, text: &str, scale: f32) -> i32 {
    let scale = Scale::uniform(scale);
    let mut width = 0.0;

    for glyph in font.layout(text, scale, rusttype::point(0.0, 0.0)) {
        if let Some(bb) = glyph.pixel_bounding_box() {
            width = bb.max.x as f32;
        } else {
            width += glyph.unpositioned().h_metrics().advance_width;
        }
    }

    width as i32
}

/// Fill a rectangle with a color
pub fn fill_rect(image: &mut RgbImage, color: Rgb<u8>) {
    for pixel in image.pixels_mut() {
        *pixel = color;
    }
}

/// Draw a filled rectangle
pub fn draw_filled_rect(
    image: &mut RgbImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    color: Rgb<u8>,
) {
    for py in y..(y + height).min(image.height()) {
        for px in x..(x + width).min(image.width()) {
            image.put_pixel(px, py, color);
        }
    }
}
