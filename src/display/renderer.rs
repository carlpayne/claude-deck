use anyhow::Result;
use image::{Rgb, RgbImage};
use rusttype::{Font, Scale};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::debug;

use crate::config::Config;
use crate::profiles::ProfileManager;
use crate::state::AppState;

use super::buttons::render_button_with_config_and_id;
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
pub const BRIGHT_ORANGE: Rgb<u8> = Rgb([255, 180, 60]);
/// Warm background for waiting-flash "on" phase
pub const WAITING_GLOW_BG: Rgb<u8> = Rgb([80, 45, 5]);
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
    profile_manager: Arc<RwLock<ProfileManager>>,
}

impl DisplayRenderer {
    pub fn new(config: &Config, profile_manager: Arc<RwLock<ProfileManager>>) -> Result<Self> {
        // Load embedded font (or fall back to system font)
        let font_data = include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf");
        let font = Font::try_from_bytes(font_data as &[u8])
            .ok_or_else(|| anyhow::anyhow!("Failed to load font"))?;

        Ok(Self {
            font,
            config: config.clone(),
            icon_cache: HashMap::new(),
            profile_manager,
        })
    }

    /// Render a button image
    pub fn render_button(&self, button_id: u8, active: bool, state: &AppState) -> Result<RgbImage> {
        use crate::profiles::ButtonAction;

        // If screen is locked, render dimmed/disabled button
        if state.screen_locked {
            return self.render_locked_button();
        }

        // Get button config from profile manager (uses configurable profiles)
        let button_config = {
            let manager = self.profile_manager.read().unwrap();
            manager.get_button_config(&state.focused_app, button_id)
        };

        // Check if this button has MIC action - needs special rendering with mic icon
        if matches!(&button_config.action, ButtonAction::Custom(action) if *action == "MIC") {
            return super::buttons::render_mic_button(
                &self.font,
                active,
                state.dictation_active,
                button_config.colors,
            );
        }

        // Use the profile-specific button configuration (with button_id for GIF animation)
        render_button_with_config_and_id(&self.font, &button_config, active, Some(button_id))
    }

    /// Render a locked/disabled button (shown when screen is locked)
    fn render_locked_button(&self) -> Result<RgbImage> {
        use crate::device::{BUTTON_HEIGHT, BUTTON_WIDTH};

        let mut img = RgbImage::new(BUTTON_WIDTH, BUTTON_HEIGHT);

        // Dark gray background
        let dark = Rgb([25, 25, 30]);
        let darker = Rgb([15, 15, 18]);
        for y in 0..BUTTON_HEIGHT {
            let t = y as f32 / BUTTON_HEIGHT as f32;
            let r = (dark[0] as f32 * (1.0 - t) + darker[0] as f32 * t) as u8;
            let g = (dark[1] as f32 * (1.0 - t) + darker[1] as f32 * t) as u8;
            let b = (dark[2] as f32 * (1.0 - t) + darker[2] as f32 * t) as u8;
            for x in 0..BUTTON_WIDTH {
                img.put_pixel(x, y, Rgb([r, g, b]));
            }
        }

        // Subtle border
        let border = Rgb([40, 40, 48]);
        for x in 0..BUTTON_WIDTH {
            img.put_pixel(x, 0, border);
            img.put_pixel(x, BUTTON_HEIGHT - 1, border);
        }
        for y in 0..BUTTON_HEIGHT {
            img.put_pixel(0, y, border);
            img.put_pixel(BUTTON_WIDTH - 1, y, border);
        }

        Ok(img)
    }

    /// Render a button with a pre-provided GIF frame (avoids animator lock)
    pub fn render_button_with_gif_frame(
        &self,
        button_id: u8,
        state: &AppState,
        gif_frame: &std::sync::Arc<image::RgbaImage>,
    ) -> Result<RgbImage> {
        // Get button config from profile manager
        let button_config = {
            let manager = self.profile_manager.read().unwrap();
            manager.get_button_config(&state.focused_app, button_id)
        };

        // Render using the provided frame (deref Arc to get &RgbaImage)
        super::buttons::render_button_with_gif_frame(&self.font, &button_config, gif_frame.as_ref())
    }

    /// Render a solid colored button (for animations)
    pub fn render_solid_button(&self, r: u8, g: u8, b: u8) -> Result<RgbImage> {
        use crate::device::{BUTTON_HEIGHT, BUTTON_WIDTH};
        let mut img = RgbImage::new(BUTTON_WIDTH, BUTTON_HEIGHT);
        let color = Rgb([r, g, b]);
        for pixel in img.pixels_mut() {
            *pixel = color;
        }
        Ok(img)
    }

    /// Render the full LCD strip (800x128)
    pub fn render_strip(&self, state: &AppState) -> Result<RgbImage> {
        render_strip_image(&self.font, state)
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
