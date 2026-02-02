use anyhow::Result;
use image::{Rgb, RgbImage};
use rusttype::Font;

use super::renderer::{
    draw_filled_rect, draw_text, fill_rect, text_width, BLUE, BRIGHT_PURPLE, DARK_BG, GRAY, GREEN,
    ORANGE, RED, WHITE,
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

    let (status, color) = if state.connected {
        ("CONNECTED", GREEN)
    } else {
        ("OFFLINE", RED)
    };

    // Status text centered
    let status_width = text_width(font, status, 15.0);
    let x = ((STRIP_BUTTON_WIDTH as i32 - status_width) / 2).max(4);
    draw_text(img, font, status, x, 45, 15.0, color);

    // Connection indicator dot
    let dot_x = (STRIP_BUTTON_WIDTH as i32 / 2) - 8;
    draw_text(img, font, "●", dot_x, 78, 24.0, color);
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

/// Render the LCD strip with status information
pub fn render_strip_image(font: &Font, state: &AppState) -> Result<RgbImage> {
    let mut img = RgbImage::new(STRIP_WIDTH, STRIP_HEIGHT);

    // Fill background
    fill_rect(&mut img, DARK_BG);

    // Draw top section: Task name and progress
    draw_task_section(&mut img, font, state);

    // Draw bottom section: Model and YOLO indicator
    draw_status_section(&mut img, font, state);

    // Draw separator line
    draw_separator(&mut img, 64);

    Ok(img)
}

/// Draw the task name and tool detail
fn draw_task_section(img: &mut RgbImage, font: &Font, state: &AppState) {
    let y_offset = 12;

    // Task label
    draw_text(img, font, "TASK:", 10, y_offset, 14.0, GRAY);

    // Task name
    let task_color = if state.task_name == "ERROR" || state.task_name == "RATE LIMITED" {
        RED
    } else if state.waiting_for_input {
        Rgb([255, 200, 0]) // Yellow for waiting
    } else {
        WHITE
    };
    draw_text(img, font, &state.task_name, 70, y_offset, 14.0, task_color);

    // Tool detail if available
    if let Some(ref detail) = state.tool_detail {
        draw_text(img, font, detail, 200, y_offset, 12.0, GRAY);
    }
}

/// Draw the model selector and YOLO indicator
fn draw_status_section(img: &mut RgbImage, font: &Font, state: &AppState) {
    let y_offset = 85;

    if state.model_selecting {
        // Show model selector with all options
        draw_model_selector(img, font, state, y_offset);
    } else {
        // Show current model
        draw_text(img, font, "MODEL:", 10, y_offset, 14.0, GRAY);
        draw_text(
            img,
            font,
            &state.model.to_uppercase(),
            85,
            y_offset,
            14.0,
            GREEN,
        );
    }

    // Connection indicator
    let conn_text = if state.connected { "●" } else { "○" };
    let conn_color = if state.connected { GREEN } else { GRAY };
    draw_text(
        img,
        font,
        conn_text,
        STRIP_WIDTH as i32 - 25,
        12,
        14.0,
        conn_color,
    );
}

/// Draw model selector with visual indicator
fn draw_model_selector(img: &mut RgbImage, font: &Font, state: &AppState, y: i32) {
    let mut x = 10;
    let scale = 14.0;

    for (i, model) in state.available_models.iter().enumerate() {
        let is_selected = i == state.model_index;
        let prefix = if is_selected { "●" } else { "○" };
        let color = if is_selected { GREEN } else { GRAY };

        let text = format!("{} {}", prefix, model);
        draw_text(img, font, &text, x, y, scale, color);

        x += text_width(font, &text, scale) + 20;
    }

    // Draw hint text
    let hint = "← rotate | press to confirm →";
    let hint_width = text_width(font, hint, 10.0);
    draw_text(
        img,
        font,
        hint,
        STRIP_WIDTH as i32 - hint_width - 10,
        y + 20,
        10.0,
        GRAY,
    );
}

/// Draw a separator line
fn draw_separator(img: &mut RgbImage, y: u32) {
    let color = Rgb([40, 40, 60]);
    for x in 10..(STRIP_WIDTH - 10) {
        img.put_pixel(x, y, color);
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
