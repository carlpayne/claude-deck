use anyhow::Result;
use image::{Rgb, RgbImage};
use rusttype::Font;

use super::renderer::{
    draw_filled_rect, draw_text, text_width, BLUE, BRIGHT_PURPLE, GRAY, GREEN, ORANGE, RED, WHITE,
};
use crate::device::{STRIP_BUTTON_HEIGHT, STRIP_BUTTON_WIDTH, STRIP_HEIGHT, STRIP_WIDTH};
use crate::state::AppState;

/// Strip button labels
pub const STRIP_BUTTON_LABELS: [&str; 4] = [
    "STATUS", // 0 - Connection/task status
    "MODEL",  // 1 - Current model
    "TASK",   // 2 - Current task
    "MIC",    // 3 - Dictation indicator
];

/// Render a single LCD strip soft button (176x124)
pub fn render_strip_button(font: &Font, button_id: u8, state: &AppState) -> Result<RgbImage> {
    let mut img = RgbImage::new(STRIP_BUTTON_WIDTH, STRIP_BUTTON_HEIGHT);

    // Fill with gradient background
    fill_gradient_vertical(&mut img, Rgb([20, 22, 32]), Rgb([12, 14, 20]));

    // Draw styled border
    draw_strip_button_border(&mut img, Rgb([50, 55, 70]), Rgb([30, 32, 42]));

    match button_id {
        0 => render_status_button(&mut img, font, state),
        1 => render_model_button(&mut img, font, state),
        2 => render_task_button(&mut img, font, state),
        3 => render_mode_button(&mut img, font, state),
        _ => {}
    }

    Ok(img)
}

/// Fill with vertical gradient
fn fill_gradient_vertical(img: &mut RgbImage, top: Rgb<u8>, bottom: Rgb<u8>) {
    let h = img.height() as f32;
    for y in 0..img.height() {
        let t = y as f32 / h;
        let r = ((top[0] as f32) * (1.0 - t) + (bottom[0] as f32) * t) as u8;
        let g = ((top[1] as f32) * (1.0 - t) + (bottom[1] as f32) * t) as u8;
        let b = ((top[2] as f32) * (1.0 - t) + (bottom[2] as f32) * t) as u8;
        for x in 0..img.width() {
            img.put_pixel(x, y, Rgb([r, g, b]));
        }
    }
}

/// Render status button (connection indicator)
fn render_status_button(img: &mut RgbImage, font: &Font, state: &AppState) {
    // Header with accent line
    draw_filled_rect(img, 4, 4, STRIP_BUTTON_WIDTH - 8, 20, Rgb([30, 35, 45]));
    draw_text(img, font, "STATUS", 10, 6, 11.0, Rgb([120, 130, 150]));

    // Show LOCKED status when screen is locked (input disabled)
    let (status, color) = if state.screen_locked {
        ("LOCKED", ORANGE)
    } else if state.connected {
        ("CONNECTED", GREEN)
    } else {
        ("OFFLINE", RED)
    };

    // Status text centered
    let status_width = text_width(font, status, 15.0);
    let x = ((STRIP_BUTTON_WIDTH as i32 - status_width) / 2).max(4);
    draw_text(img, font, status, x, 45, 15.0, color);

    // Connection indicator dot (or lock symbol when locked)
    let dot_x = (STRIP_BUTTON_WIDTH as i32 / 2) - 8;
    if state.screen_locked {
        // Draw a simple lock symbol using ASCII
        draw_text(img, font, "[X]", dot_x - 8, 78, 18.0, ORANGE);
    } else {
        draw_text(img, font, "●", dot_x, 78, 24.0, color);
    }
}

/// Render model button (current model)
fn render_model_button(img: &mut RgbImage, font: &Font, state: &AppState) {
    // Header
    draw_filled_rect(img, 4, 4, STRIP_BUTTON_WIDTH - 8, 20, Rgb([30, 35, 45]));
    draw_text(img, font, "MODEL", 10, 6, 11.0, Rgb([120, 130, 150]));

    let model_upper = state.model.to_uppercase();

    if state.model_selecting {
        // Selection mode - show with highlight
        draw_filled_rect(img, 8, 38, STRIP_BUTTON_WIDTH - 16, 35, Rgb([20, 50, 35]));
        let model_width = text_width(font, &model_upper, 20.0);
        let x = ((STRIP_BUTTON_WIDTH as i32 - model_width) / 2).max(4);
        draw_text(img, font, &model_upper, x, 42, 20.0, BRIGHT_PURPLE);

        // Rotation hint
        draw_text(img, font, "< rotate >", 35, 85, 10.0, Rgb([80, 90, 110]));
    } else {
        let model_width = text_width(font, &model_upper, 22.0);
        let x = ((STRIP_BUTTON_WIDTH as i32 - model_width) / 2).max(4);
        draw_text(img, font, &model_upper, x, 48, 22.0, BLUE);
    }
}

/// Render task button (current task)
fn render_task_button(img: &mut RgbImage, font: &Font, state: &AppState) {
    // Header
    draw_filled_rect(img, 4, 4, STRIP_BUTTON_WIDTH - 8, 20, Rgb([30, 35, 45]));
    draw_text(img, font, "TASK", 10, 6, 11.0, Rgb([120, 130, 150]));

    let task_color = if state.task_name == "ERROR" || state.task_name == "RATE LIMITED" {
        RED
    } else if state.waiting_for_input {
        ORANGE
    } else if state.task_name == "READY" {
        GREEN
    } else if state.task_name == "THINKING" {
        BRIGHT_PURPLE
    } else {
        WHITE
    };

    // Line 1: Task/status name (centered)
    let task = if state.task_name.len() > 12 {
        format!("{}...", &state.task_name[..9])
    } else {
        state.task_name.clone()
    };

    let task_width = text_width(font, &task, 14.0);
    let x = ((STRIP_BUTTON_WIDTH as i32 - task_width) / 2).max(4);
    draw_text(img, font, &task, x, 32, 14.0, task_color);

    // Line 2: Tool detail (file/command preview)
    if let Some(ref detail) = state.tool_detail {
        let detail_str = if detail.len() > 14 {
            format!("{}...", &detail[..11])
        } else {
            detail.clone()
        };
        let detail_width = text_width(font, &detail_str, 11.0);
        let x = ((STRIP_BUTTON_WIDTH as i32 - detail_width) / 2).max(4);
        draw_text(img, font, &detail_str, x, 55, 11.0, GRAY);
    }

    // Line 3: Status indicator
    if state.waiting_for_input {
        let wait_width = text_width(font, "WAITING", 10.0);
        let x = ((STRIP_BUTTON_WIDTH as i32 - wait_width) / 2).max(4);
        draw_text(img, font, "WAITING", x, 78, 10.0, ORANGE);
    } else if state.task_name == "THINKING" {
        // Animated dots would be nice, but for now just show dots
        let dots_width = text_width(font, "...", 12.0);
        let x = ((STRIP_BUTTON_WIDTH as i32 - dots_width) / 2).max(4);
        draw_text(img, font, "...", x, 78, 12.0, BRIGHT_PURPLE);
    }
}

/// Render mic/dictation button
fn render_mode_button(img: &mut RgbImage, font: &Font, state: &AppState) {
    // Header
    draw_filled_rect(img, 4, 4, STRIP_BUTTON_WIDTH - 8, 20, Rgb([30, 35, 45]));
    draw_text(img, font, "MIC", 10, 6, 11.0, Rgb([120, 130, 150]));

    if state.dictation_active {
        // Recording - red styling
        draw_filled_rect(img, 8, 35, STRIP_BUTTON_WIDTH - 16, 45, Rgb([50, 15, 15]));
        let rec_width = text_width(font, "REC", 22.0);
        let x = ((STRIP_BUTTON_WIDTH as i32 - rec_width) / 2).max(4);
        draw_text(img, font, "REC", x, 42, 22.0, RED);
        draw_text(img, font, "recording...", 28, 85, 10.0, RED);
    } else {
        let ready_width = text_width(font, "READY", 18.0);
        let x = ((STRIP_BUTTON_WIDTH as i32 - ready_width) / 2).max(4);
        draw_text(img, font, "READY", x, 48, 18.0, GRAY);
        draw_text(img, font, "press MIC", 32, 85, 10.0, Rgb([80, 90, 100]));
    }
}

/// Draw styled border around strip button (3D effect)
fn draw_strip_button_border(img: &mut RgbImage, highlight: Rgb<u8>, shadow: Rgb<u8>) {
    let w = img.width();
    let h = img.height();

    // Top edge (highlight)
    for x in 0..w {
        img.put_pixel(x, 0, highlight);
        img.put_pixel(x, 1, highlight);
    }

    // Left edge (highlight)
    for y in 0..h {
        img.put_pixel(0, y, highlight);
        img.put_pixel(1, y, highlight);
    }

    // Bottom edge (shadow)
    for x in 0..w {
        img.put_pixel(x, h - 1, shadow);
        img.put_pixel(x, h - 2, shadow);
    }

    // Right edge (shadow)
    for y in 0..h {
        img.put_pixel(w - 1, y, shadow);
        img.put_pixel(w - 2, y, shadow);
    }
}

/// Debug: draw outline box (keep for future debugging)
#[allow(dead_code)]
fn draw_debug_box(img: &mut RgbImage, x: u32, y: u32, w: u32, h: u32, color: Rgb<u8>) {
    // Top and bottom edges
    for px in x..(x + w).min(img.width()) {
        if y < img.height() {
            img.put_pixel(px, y, color);
        }
        if y + h - 1 < img.height() {
            img.put_pixel(px, y + h - 1, color);
        }
    }
    // Left and right edges
    for py in y..(y + h).min(img.height()) {
        if x < img.width() {
            img.put_pixel(x, py, color);
        }
        if x + w - 1 < img.width() {
            img.put_pixel(x + w - 1, py, color);
        }
    }
}

// Layout constants for 4-quadrant design
const QUAD_WIDTH: i32 = 400;   // Half of 800
const QUAD_HEIGHT: i32 = 64;   // Half of 128
const LABEL_SIZE: f32 = 14.0;  // Consistent label size
const VALUE_SIZE: f32 = 24.0;  // Consistent value size
const PADDING: i32 = 15;       // Edge padding

/// Render the LCD strip with status information (800x128)
pub fn render_strip_image(font: &Font, state: &AppState) -> Result<RgbImage> {
    let mut img = RgbImage::new(STRIP_WIDTH, STRIP_HEIGHT);

    // Fill background with subtle gradient
    fill_gradient_vertical(&mut img, Rgb([18, 20, 28]), Rgb([12, 14, 20]));

    // Draw horizontal separator
    draw_separator(&mut img, QUAD_HEIGHT as u32);

    // Draw vertical separator
    draw_vertical_separator(&mut img, QUAD_WIDTH as u32);

    // Four quadrants:
    // Top-left: Task name
    draw_quadrant_task(&mut img, font, state);
    // Top-right: Tool detail
    draw_quadrant_detail(&mut img, font, state);
    // Bottom-left: Model
    draw_quadrant_model(&mut img, font, state);
    // Bottom-right: Status
    draw_quadrant_status(&mut img, font, state);

    Ok(img)
}

/// Draw vertical separator line
fn draw_vertical_separator(img: &mut RgbImage, x: u32) {
    let color = Rgb([45, 50, 65]);
    for y in 10..(STRIP_HEIGHT - 10) {
        img.put_pixel(x, y, color);
        img.put_pixel(x + 1, y, Rgb([25, 28, 38])); // Shadow
    }
}

/// Top-left quadrant: Task name
fn draw_quadrant_task(img: &mut RgbImage, font: &Font, state: &AppState) {
    let x = PADDING;
    let y_label = 8;
    let y_value = 28;
    let max_width = QUAD_WIDTH - PADDING * 2 - 10;

    // Label
    draw_text(img, font, "TASK", x, y_label, LABEL_SIZE, GRAY);

    // Value with color based on state
    let task_color = if state.task_name == "ERROR" || state.task_name == "RATE LIMITED" {
        RED
    } else if state.waiting_for_input {
        ORANGE
    } else if state.task_name == "THINKING" {
        BRIGHT_PURPLE
    } else if state.task_name == "READY" {
        GREEN
    } else {
        WHITE
    };

    let task_display = truncate_text(font, &state.task_name, VALUE_SIZE, max_width);
    draw_text(img, font, &task_display, x, y_value, VALUE_SIZE, task_color);
}

/// Top-right quadrant: Tool detail
fn draw_quadrant_detail(img: &mut RgbImage, font: &Font, state: &AppState) {
    let x = QUAD_WIDTH + PADDING;
    let y_label = 8;
    let y_value = 28;
    // Full width available for detail text (less padding)
    let max_width = QUAD_WIDTH - PADDING - 5;

    // Label
    draw_text(img, font, "DETAIL", x, y_label, LABEL_SIZE, GRAY);

    // Value
    if let Some(ref detail) = state.tool_detail {
        let detail_display = truncate_text_path(font, detail, VALUE_SIZE, max_width);
        draw_text(img, font, &detail_display, x, y_value, VALUE_SIZE, WHITE);
    } else {
        draw_text(img, font, "-", x, y_value, VALUE_SIZE, GRAY);
    }
}

/// Bottom-left quadrant: Model
fn draw_quadrant_model(img: &mut RgbImage, font: &Font, state: &AppState) {
    let x = PADDING;
    let y_label = QUAD_HEIGHT + 6;
    let y_value = QUAD_HEIGHT + 26;

    if state.model_selecting {
        draw_text(img, font, "SELECT MODEL", x, y_label, LABEL_SIZE, GRAY);
        draw_model_selector_compact(img, font, state, x, y_value);
    } else {
        draw_text(img, font, "MODEL", x, y_label, LABEL_SIZE, GRAY);
        draw_text(img, font, &state.model.to_uppercase(), x, y_value, VALUE_SIZE, BLUE);
    }
}

/// Bottom-right quadrant: Status/hints
fn draw_quadrant_status(img: &mut RgbImage, font: &Font, state: &AppState) {
    let x = QUAD_WIDTH + PADDING;
    let y_label = QUAD_HEIGHT + 6;
    let y_value = QUAD_HEIGHT + 26;

    // Label
    draw_text(img, font, "STATUS", x, y_label, LABEL_SIZE, GRAY);

    // Status value
    let (status_text, status_color) = if state.screen_locked {
        ("LOCKED", ORANGE)
    } else if state.model_selecting {
        ("rotate to select", GRAY)
    } else if state.waiting_for_input {
        ("WAITING FOR INPUT", ORANGE)
    } else if state.connected {
        ("CONNECTED", GREEN)
    } else {
        ("OFFLINE", RED)
    };

    draw_text(img, font, status_text, x, y_value, VALUE_SIZE, status_color);
}

/// Compact model selector for bottom-left quadrant
fn draw_model_selector_compact(img: &mut RgbImage, font: &Font, state: &AppState, start_x: i32, y: i32) {
    let mut x = start_x;
    let scale = 18.0;
    let spacing = 15;
    let max_x = QUAD_WIDTH - PADDING;

    for (i, model) in state.available_models.iter().enumerate() {
        let is_selected = i == state.model_index;
        let color = if is_selected { GREEN } else { GRAY };
        let model_upper = model.to_uppercase();
        let model_width = text_width(font, &model_upper, scale);

        if x + model_width > max_x {
            break;
        }

        if is_selected {
            draw_filled_rect(img, x as u32 - 3, y as u32 - 2, model_width as u32 + 6, 24, Rgb([30, 50, 40]));
        }

        draw_text(img, font, &model_upper, x, y, scale, color);
        x += model_width + spacing;
    }
}

/// Truncate text to fit width, adding ".." if needed
fn truncate_text(font: &Font, text: &str, scale: f32, max_width: i32) -> String {
    let mut display = text.to_string();
    while text_width(font, &display, scale) > max_width && display.len() > 3 {
        display.pop();
    }
    if display.len() < text.len() {
        if display.len() > 2 {
            display.pop();
            display.pop();
        }
        display.push_str("..");
    }
    display
}

/// Truncate path, keeping filename visible
fn truncate_text_path(font: &Font, text: &str, scale: f32, max_width: i32) -> String {
    if text_width(font, text, scale) <= max_width {
        return text.to_string();
    }

    // For paths, try to show the end (filename)
    if let Some(idx) = text.rfind('/') {
        let filename = &text[idx..];
        if text_width(font, filename, scale) <= max_width {
            let prefix = format!("..{}", filename);
            if text_width(font, &prefix, scale) <= max_width {
                return prefix;
            }
        }
    }

    truncate_text(font, text, scale, max_width)
}

/// Draw a horizontal separator line
fn draw_separator(img: &mut RgbImage, y: u32) {
    let color = Rgb([45, 50, 65]);
    for x in 15..(STRIP_WIDTH - 15) {
        img.put_pixel(x, y, color);
        img.put_pixel(x, y + 1, Rgb([25, 28, 38])); // Shadow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_strip() {
        let font_data = include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf");
        let font = Font::try_from_bytes(font_data as &[u8]).unwrap();

        let state = AppState::new();
        let img = render_strip_image(&font, &state).unwrap();

        assert_eq!(img.width(), STRIP_WIDTH);
        assert_eq!(img.height(), STRIP_HEIGHT);
    }
}
