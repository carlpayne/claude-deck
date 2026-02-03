use anyhow::Result;
use image::{Rgb, RgbImage};
use rusttype::Font;
use std::collections::HashMap;
use std::sync::Mutex;

use super::renderer::{button_colors, draw_text, text_width, WHITE};
use crate::device::{BUTTON_HEIGHT, BUTTON_WIDTH};
use crate::profiles::ButtonConfig;

/// Cache for button backgrounds (gradient + border) keyed by color
/// Stores raw pixel data to enable fast memcpy instead of clone
static BACKGROUND_CACHE: std::sync::OnceLock<Mutex<HashMap<(u8, u8, u8), Vec<u8>>>> =
    std::sync::OnceLock::new();

/// Get or create a button with cached background for the given base color
/// Returns a new image with the background already rendered (fast memcpy)
fn get_button_with_background(base_color: Rgb<u8>) -> RgbImage {
    let cache = BACKGROUND_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = (base_color[0], base_color[1], base_color[2]);

    if let Ok(mut guard) = cache.lock() {
        if let Some(raw_data) = guard.get(&key) {
            // Fast path: create image from cached raw bytes (just memcpy)
            return RgbImage::from_raw(BUTTON_WIDTH, BUTTON_HEIGHT, raw_data.clone())
                .unwrap_or_else(|| {
                    let mut img = RgbImage::new(BUTTON_WIDTH, BUTTON_HEIGHT);
                    fill_gradient(&mut img, darken(base_color, 0.4), darken(base_color, 0.6));
                    draw_styled_border(&mut img, base_color, false);
                    img
                });
        }

        // Create new background and cache raw bytes
        let mut img = RgbImage::new(BUTTON_WIDTH, BUTTON_HEIGHT);
        fill_gradient(&mut img, darken(base_color, 0.4), darken(base_color, 0.6));
        draw_styled_border(&mut img, base_color, false);

        guard.insert(key, img.as_raw().clone());
        img
    } else {
        // Fallback if lock fails - create without caching
        let mut img = RgbImage::new(BUTTON_WIDTH, BUTTON_HEIGHT);
        fill_gradient(&mut img, darken(base_color, 0.4), darken(base_color, 0.6));
        draw_styled_border(&mut img, base_color, false);
        img
    }
}

/// Render a colored button with gradient effect
pub fn render_button_image(
    font: &Font,
    label: &str,
    active: bool,
    button_id: u8,
) -> Result<RgbImage> {
    let mut img = RgbImage::new(BUTTON_WIDTH, BUTTON_HEIGHT);

    let (base_color, bright_color) = button_colors(button_id);

    // Fill with gradient background
    if active {
        fill_gradient(&mut img, bright_color, base_color);
    } else {
        fill_gradient(&mut img, darken(base_color, 0.4), darken(base_color, 0.6));
    }

    // Draw colored border (thicker on top for 3D effect)
    let border_color = if active { bright_color } else { base_color };
    draw_styled_border(&mut img, border_color, active);

    // Calculate text positioning
    let label_scale = if label.len() <= 4 {
        20.0
    } else if label.len() <= 6 {
        16.0
    } else {
        13.0
    };
    let label_width = text_width(font, label, label_scale);
    let label_x = ((BUTTON_WIDTH as i32 - label_width) / 2).max(2);
    let label_y = (BUTTON_HEIGHT as i32 / 2) - (label_scale as i32 / 2);

    // Draw text with slight shadow for depth
    let text_color = if active { WHITE } else { Rgb([220, 220, 230]) };
    draw_text(
        &mut img,
        font,
        label,
        label_x + 1,
        label_y + 1,
        label_scale,
        Rgb([0, 0, 0]),
    ); // shadow
    draw_text(
        &mut img,
        font,
        label,
        label_x,
        label_y,
        label_scale,
        text_color,
    );

    Ok(img)
}

/// Fill image with vertical gradient (top to bottom)
fn fill_gradient(img: &mut RgbImage, top_color: Rgb<u8>, bottom_color: Rgb<u8>) {
    let h = img.height() as f32;
    for y in 0..img.height() {
        let t = y as f32 / h;
        let r = lerp(top_color[0], bottom_color[0], t);
        let g = lerp(top_color[1], bottom_color[1], t);
        let b = lerp(top_color[2], bottom_color[2], t);
        let color = Rgb([r, g, b]);
        for x in 0..img.width() {
            img.put_pixel(x, y, color);
        }
    }
}

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 * (1.0 - t) + b as f32 * t) as u8
}

fn darken(color: Rgb<u8>, factor: f32) -> Rgb<u8> {
    Rgb([
        (color[0] as f32 * factor) as u8,
        (color[1] as f32 * factor) as u8,
        (color[2] as f32 * factor) as u8,
    ])
}

/// Render a button with custom background color (for special states like recording)
pub fn render_button_with_color(
    font: &Font,
    label: &str,
    active: bool,
    _button_id: u8,
    override_color: Rgb<u8>,
) -> Result<RgbImage> {
    let mut img = RgbImage::new(BUTTON_WIDTH, BUTTON_HEIGHT);

    // Fill with gradient using override color
    let bright = brighten(override_color, 1.3);
    fill_gradient(&mut img, bright, override_color);

    // Draw styled border
    draw_styled_border(&mut img, bright, active);

    // Calculate text positioning
    let label_scale = if label.len() <= 4 {
        20.0
    } else if label.len() <= 6 {
        16.0
    } else {
        13.0
    };
    let label_width = text_width(font, label, label_scale);
    let label_x = ((BUTTON_WIDTH as i32 - label_width) / 2).max(2);
    let label_y = (BUTTON_HEIGHT as i32 / 2) - (label_scale as i32 / 2);

    // Draw text with shadow
    draw_text(
        &mut img,
        font,
        label,
        label_x + 1,
        label_y + 1,
        label_scale,
        Rgb([0, 0, 0]),
    );
    draw_text(&mut img, font, label, label_x, label_y, label_scale, WHITE);

    Ok(img)
}

/// Draw a styled border with 3D effect
fn draw_styled_border(img: &mut RgbImage, color: Rgb<u8>, active: bool) {
    let w = img.width();
    let h = img.height();

    // Outer dark border
    let dark = Rgb([20, 20, 30]);
    for x in 0..w {
        img.put_pixel(x, 0, dark);
        img.put_pixel(x, h - 1, dark);
    }
    for y in 0..h {
        img.put_pixel(0, y, dark);
        img.put_pixel(w - 1, y, dark);
    }

    // Inner colored border (brighter on top-left for 3D)
    let thickness = if active { 3 } else { 2 };
    let highlight = if active { brighten(color, 1.2) } else { color };

    // Top edge (bright)
    for x in 1..w - 1 {
        for t in 1..=thickness {
            img.put_pixel(x, t, highlight);
        }
    }

    // Left edge (bright)
    for y in 1..h - 1 {
        for t in 1..=thickness {
            img.put_pixel(t, y, highlight);
        }
    }

    // Bottom and right edges (darker for depth)
    let shadow = darken(color, 0.6);
    for x in 1..w - 1 {
        for t in 1..=thickness {
            img.put_pixel(x, h - 1 - t, shadow);
        }
    }
    for y in 1..h - 1 {
        for t in 1..=thickness {
            img.put_pixel(w - 1 - t, y, shadow);
        }
    }
}

fn brighten(color: Rgb<u8>, factor: f32) -> Rgb<u8> {
    Rgb([
        (color[0] as f32 * factor).min(255.0) as u8,
        (color[1] as f32 * factor).min(255.0) as u8,
        (color[2] as f32 * factor).min(255.0) as u8,
    ])
}

/// Load a GIF from URL and return the first frame as RgbaImage
/// Uses a simple cache to avoid repeated fetches
fn load_gif_image(url: &str) -> Option<image::RgbaImage> {
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::io::Read;

    // Simple in-memory cache for fetched GIFs
    static GIF_CACHE: std::sync::OnceLock<Mutex<HashMap<String, Option<image::RgbaImage>>>> =
        std::sync::OnceLock::new();

    let cache = GIF_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache_guard = cache.lock().ok()?;

    // Check cache first
    if let Some(cached) = cache_guard.get(url) {
        return cached.clone();
    }

    // Fetch the GIF
    let result = (|| -> Option<image::RgbaImage> {
        let response = ureq::get(url).call().ok()?;

        // Read response body
        let mut bytes = Vec::new();
        response.into_reader().take(5_000_000).read_to_end(&mut bytes).ok()?; // 5MB limit

        // Load as image (handles GIF first frame automatically)
        let img = image::load_from_memory(&bytes).ok()?;
        Some(img.to_rgba8())
    })();

    // Cache the result (even if None, to avoid repeated failed fetches)
    cache_guard.insert(url.to_string(), result.clone());
    result
}

/// Render an RGBA image centered on the button
fn render_image_on_button(img: &mut RgbImage, source: &image::RgbaImage) {
    let image_size = 90u32; // Target size (buttons are 112x112)

    // Skip resize if image is already the target size (e.g., pre-resized GIF frames)
    if source.width() == image_size && source.height() == image_size {
        render_presized_image_on_button(img, source);
        return;
    }

    let resized = image::imageops::resize(
        source,
        image_size,
        image_size,
        image::imageops::FilterType::Triangle, // Fast bilinear instead of slow Lanczos3
    );

    render_presized_image_on_button(img, &resized);
}

/// Render a pre-sized 90x90 image centered on the button (fast path)
/// Uses direct buffer access for better performance
#[inline]
fn render_presized_image_on_button(img: &mut RgbImage, source: &image::RgbaImage) {
    let src_width = source.width() as usize;
    let src_height = source.height() as usize;
    let x_offset = ((BUTTON_WIDTH - source.width()) / 2) as usize;
    let y_offset = ((BUTTON_HEIGHT - source.height()) / 2) as usize;
    let dst_width = BUTTON_WIDTH as usize;

    let src_raw = source.as_raw();
    let dst_raw = img.as_mut();

    // Direct buffer access - much faster than per-pixel put_pixel
    for sy in 0..src_height {
        let dy = sy + y_offset;
        let src_row_start = sy * src_width * 4;
        let dst_row_start = dy * dst_width * 3;

        for sx in 0..src_width {
            let src_idx = src_row_start + sx * 4;
            let alpha = src_raw[src_idx + 3];

            // Skip transparent pixels
            if alpha > 128 {
                let dx = sx + x_offset;
                let dst_idx = dst_row_start + dx * 3;

                dst_raw[dst_idx] = src_raw[src_idx];         // R
                dst_raw[dst_idx + 1] = src_raw[src_idx + 1]; // G
                dst_raw[dst_idx + 2] = src_raw[src_idx + 2]; // B
            }
        }
    }
}

/// Render a button with profile-specific configuration
pub fn render_button_with_config(
    font: &Font,
    config: &ButtonConfig,
    active: bool,
) -> Result<RgbImage> {
    render_button_with_config_and_id(font, config, active, None)
}

/// Render a button with a pre-provided GIF frame (fast path for animation)
/// Uses cached background for maximum performance
pub fn render_button_with_gif_frame(
    _font: &Font,
    config: &ButtonConfig,
    gif_frame: &image::RgbaImage,
) -> Result<RgbImage> {
    let (base_color, _bright_color) = config.colors;

    // Get button with cached background (gradient + border) - fast memcpy
    let mut img = get_button_with_background(base_color);

    // Render the pre-provided GIF frame (already pre-sized)
    render_presized_image_on_button(&mut img, gif_frame);

    Ok(img)
}

/// Render a button with profile-specific configuration and button ID for GIF animation
pub fn render_button_with_config_and_id(
    font: &Font,
    config: &ButtonConfig,
    active: bool,
    button_id: Option<u8>,
) -> Result<RgbImage> {
    let mut img = RgbImage::new(BUTTON_WIDTH, BUTTON_HEIGHT);

    let (base_color, bright_color) = config.colors;

    // Fill with gradient background
    if active {
        fill_gradient(&mut img, bright_color, base_color);
    } else {
        fill_gradient(&mut img, darken(base_color, 0.4), darken(base_color, 0.6));
    }

    // Draw colored border (thicker on top for 3D effect)
    let border_color = if active { bright_color } else { base_color };
    draw_styled_border(&mut img, border_color, active);

    // Priority: gif_url > custom_image > emoji_image > text label
    let image_rendered = if let Some(gif_url) = config.gif_url {
        // GIF from URL - use animated frame if available
        let mut frame_found = false;

        if let Some(btn_id) = button_id {
            let animator = super::gif::animator();
            let lock_result = animator.lock();
            if let Ok(mut anim) = lock_result {
                // Ensure animation is set up for this button
                if !anim.has_animation(btn_id) {
                    anim.set_button_gif(btn_id, gif_url);
                }

                // Get current animation frame
                if let Some(frame_img) = anim.get_current_frame(btn_id) {
                    render_image_on_button(&mut img, frame_img);
                    frame_found = true;
                }
            }
        }

        // Fallback to static first frame
        if !frame_found {
            if let Some(gif_img) = load_gif_image(gif_url) {
                render_image_on_button(&mut img, &gif_img);
                frame_found = true;
            }
        }

        frame_found
    } else if let Some(custom_image) = config.custom_image {
        // Custom image from base64 data URL
        if let Some(rgba_img) = super::emoji::load_base64_image(custom_image) {
            render_image_on_button(&mut img, &rgba_img);
            true
        } else {
            false
        }
    } else if let Some(emoji_ref) = config.emoji_image {
        // Emoji from Twemoji
        if let Some(emoji_img) = super::emoji::get_emoji_image(emoji_ref) {
            render_image_on_button(&mut img, &emoji_img);
            true
        } else {
            false
        }
    } else {
        false
    };

    if !image_rendered {
        // Render text label if no emoji image
        let label = config.label;
        let label_scale = if label.len() <= 4 {
            20.0
        } else if label.len() <= 6 {
            16.0
        } else {
            13.0
        };
        let label_width = text_width(font, label, label_scale);
        let label_x = ((BUTTON_WIDTH as i32 - label_width) / 2).max(2);
        let label_y = (BUTTON_HEIGHT as i32 / 2) - (label_scale as i32 / 2);

        // Draw text with slight shadow for depth
        let text_color = if active { WHITE } else { Rgb([220, 220, 230]) };
        draw_text(
            &mut img,
            font,
            label,
            label_x + 1,
            label_y + 1,
            label_scale,
            Rgb([0, 0, 0]),
        ); // shadow
        draw_text(
            &mut img,
            font,
            label,
            label_x,
            label_y,
            label_scale,
            text_color,
        );
    }

    Ok(img)
}

/// Render a MIC button with microphone icon
pub fn render_mic_button(
    font: &Font,
    active: bool,
    recording: bool,
    colors: (Rgb<u8>, Rgb<u8>),
) -> Result<RgbImage> {
    let mut img = RgbImage::new(BUTTON_WIDTH, BUTTON_HEIGHT);

    let (base_color, bright_color) = if recording {
        (Rgb([180, 50, 50]), Rgb([220, 70, 70])) // Red when recording
    } else {
        colors
    };

    // Fill with gradient background
    if active || recording {
        fill_gradient(&mut img, bright_color, base_color);
    } else {
        fill_gradient(&mut img, darken(base_color, 0.4), darken(base_color, 0.6));
    }

    // Draw styled border
    let border_color = if active || recording {
        bright_color
    } else {
        base_color
    };
    draw_styled_border(&mut img, border_color, active || recording);

    // Draw microphone icon
    let icon_color = if active || recording {
        WHITE
    } else {
        Rgb([220, 220, 230])
    };
    let shadow_color = Rgb([0, 0, 0]);

    draw_mic_icon(&mut img, shadow_color, 1, 1, recording); // Shadow
    draw_mic_icon(&mut img, icon_color, 0, 0, recording); // Icon

    // Draw "REC" label below icon if recording
    if recording {
        let rec_width = text_width(font, "REC", 14.0);
        let rec_x = ((BUTTON_WIDTH as i32 - rec_width) / 2).max(2);
        draw_text(&mut img, font, "REC", rec_x, 88, 14.0, WHITE);
    }

    Ok(img)
}

/// Draw a microphone icon
fn draw_mic_icon(img: &mut RgbImage, color: Rgb<u8>, offset_x: i32, offset_y: i32, small: bool) {
    let cx = (BUTTON_WIDTH / 2) as i32 + offset_x;
    // Move icon up more when small (recording mode) to make room for REC text
    let cy = (BUTTON_HEIGHT / 2) as i32 + offset_y - if small { 18 } else { 8 };

    // Mic body (rounded rectangle) - 20x32 pixels
    let mic_width = 20;
    let mic_height = 32;
    let mic_left = cx - mic_width / 2;
    let mic_top = cy - mic_height / 2;

    // Draw mic body with rounded top
    for y in mic_top..(mic_top + mic_height) {
        for x in mic_left..(mic_left + mic_width) {
            if x >= 0 && x < BUTTON_WIDTH as i32 && y >= 0 && y < BUTTON_HEIGHT as i32 {
                // Round the top corners
                let rel_y = y - mic_top;
                let rel_x = x - mic_left;
                let corner_radius = 8;

                let in_body = if rel_y < corner_radius {
                    // Top rounded part
                    let dx = if rel_x < corner_radius {
                        corner_radius - rel_x
                    } else if rel_x >= mic_width - corner_radius {
                        rel_x - (mic_width - corner_radius - 1)
                    } else {
                        0
                    };
                    let dy = corner_radius - rel_y;
                    dx * dx + dy * dy <= corner_radius * corner_radius
                } else {
                    true
                };

                if in_body {
                    img.put_pixel(x as u32, y as u32, color);
                }
            }
        }
    }

    // Mic grille lines (3 horizontal lines)
    let grille_color = darken(color, 0.6);
    for i in 0..3 {
        let line_y = mic_top + 10 + i * 6;
        if line_y >= 0 && line_y < BUTTON_HEIGHT as i32 {
            for x in (mic_left + 4)..(mic_left + mic_width - 4) {
                if x >= 0 && x < BUTTON_WIDTH as i32 {
                    img.put_pixel(x as u32, line_y as u32, grille_color);
                }
            }
        }
    }

    // Stand arc (curved line under mic)
    let arc_y = mic_top + mic_height + 2;
    let arc_width = 28;
    let arc_left = cx - arc_width / 2;

    for x in arc_left..(arc_left + arc_width) {
        if x >= 0 && x < BUTTON_WIDTH as i32 {
            let rel_x = (x - cx) as f32;
            let arc_height = ((arc_width as f32 / 2.0).powi(2) - rel_x.powi(2)).sqrt() * 0.4;
            let y = arc_y + arc_height as i32;
            if y >= 0 && y < BUTTON_HEIGHT as i32 {
                img.put_pixel(x as u32, y as u32, color);
                if y + 1 < BUTTON_HEIGHT as i32 {
                    img.put_pixel(x as u32, (y + 1) as u32, color);
                }
            }
        }
    }

    // Stand stem (vertical line down from arc)
    let stem_top = arc_y + 8;
    let stem_bottom = stem_top + 12;
    for y in stem_top..stem_bottom {
        if y >= 0 && y < BUTTON_HEIGHT as i32 {
            img.put_pixel(cx as u32, y as u32, color);
            img.put_pixel((cx + 1) as u32, y as u32, color);
        }
    }

    // Stand base (horizontal line)
    let base_y = stem_bottom;
    let base_width = 20;
    if base_y >= 0 && base_y < BUTTON_HEIGHT as i32 {
        for x in (cx - base_width / 2)..(cx + base_width / 2) {
            if x >= 0 && x < BUTTON_WIDTH as i32 {
                img.put_pixel(x as u32, base_y as u32, color);
                if base_y + 1 < BUTTON_HEIGHT as i32 {
                    img.put_pixel(x as u32, (base_y + 1) as u32, color);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_button() {
        // Load a basic font for testing
        let font_data = include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf");
        let font = Font::try_from_bytes(font_data as &[u8]).unwrap();

        let img = render_button_image(&font, "TEST", false, 0).unwrap();
        assert_eq!(img.width(), BUTTON_WIDTH);
        assert_eq!(img.height(), BUTTON_HEIGHT);
    }
}
