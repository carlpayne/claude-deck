//! App profiles for context-aware button configurations

use image::Rgb;

use crate::display::renderer::{
    BLUE, BRIGHT_BLUE, BRIGHT_GRAY, BRIGHT_GREEN, BRIGHT_PURPLE, BRIGHT_RED, GRAY, GREEN, ORANGE,
    PURPLE, RED,
};
use crate::input::keystrokes::Key;

/// Application profile types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppProfile {
    /// Default Claude Code mode
    Claude,
    /// Slack emoji shortcuts mode
    Slack,
}

/// Action to perform when a button is pressed
#[derive(Debug, Clone)]
pub enum ButtonAction {
    /// Send a keyboard key
    Key(Key),
    /// Type text directly
    Text(String),
    /// Slack emoji shortcode (types `:emoji:`)
    SlackEmoji(String),
    /// Custom action handled by the input handler
    Custom(&'static str),
}

/// Button configuration for rendering and actions
#[derive(Debug, Clone)]
pub struct ButtonConfig {
    pub label: &'static str,
    pub colors: (Rgb<u8>, Rgb<u8>),
    pub action: ButtonAction,
    /// Optional emoji image name (without extension)
    pub emoji_image: Option<&'static str>,
}

/// Get the appropriate profile for an application name
pub fn get_profile_for_app(app_name: &str) -> AppProfile {
    match app_name {
        "Slack" => AppProfile::Slack,
        _ => AppProfile::Claude,
    }
}

/// Slack button definition tuple type: (label, shortcode, colors, image_name)
type SlackButtonDef = (&'static str, &'static str, (Rgb<u8>, Rgb<u8>), &'static str);

/// Slack button configurations - emoji shortcuts
const SLACK_BUTTONS: [SlackButtonDef; 10] = [
    // Position 0-4: Top row
    (
        "👍",
        ":+1:",
        (Rgb([255, 200, 50]), Rgb([255, 220, 100])),
        "thumbsup",
    ),
    (
        "👎",
        ":-1:",
        (Rgb([100, 100, 120]), Rgb([130, 130, 150])),
        "thumbsdown",
    ),
    ("✅", ":white_check_mark:", (GREEN, BRIGHT_GREEN), "check"),
    ("👀", ":eyes:", (PURPLE, BRIGHT_PURPLE), "eyes"),
    (
        "🎉",
        ":tada:",
        (Rgb([220, 100, 180]), Rgb([255, 130, 210])),
        "tada",
    ),
    // Position 5-9: Bottom row
    ("❤️", ":heart:", (RED, BRIGHT_RED), "heart"),
    (
        "😂",
        ":joy:",
        (Rgb([255, 200, 50]), Rgb([255, 220, 100])),
        "joy",
    ),
    ("🔥", ":fire:", (ORANGE, Rgb([255, 180, 80])), "fire"),
    ("💯", ":100:", (RED, BRIGHT_RED), "hundred"),
    ("🙏", ":pray:", (BLUE, BRIGHT_BLUE), "pray"),
];

impl AppProfile {
    /// Get button configuration for a specific button ID
    pub fn button_config(&self, button_id: u8) -> ButtonConfig {
        match self {
            AppProfile::Slack => {
                let idx = button_id as usize;
                if idx < SLACK_BUTTONS.len() {
                    let (label, emoji, colors, image) = SLACK_BUTTONS[idx];
                    ButtonConfig {
                        label,
                        colors,
                        action: ButtonAction::SlackEmoji(emoji.to_string()),
                        emoji_image: Some(image),
                    }
                } else {
                    // Fallback for any unmapped buttons
                    ButtonConfig {
                        label: "?",
                        colors: (GRAY, BRIGHT_GRAY),
                        action: ButtonAction::Text("".to_string()),
                        emoji_image: None,
                    }
                }
            }
            AppProfile::Claude => {
                // Claude mode uses default button rendering
                // Return config with Custom action to indicate default handling
                let (label, colors) = claude_button_config(button_id);
                ButtonConfig {
                    label,
                    colors,
                    action: ButtonAction::Custom(label),
                    emoji_image: None,
                }
            }
        }
    }
}

/// Get Claude mode button configuration (label and colors)
fn claude_button_config(button_id: u8) -> (&'static str, (Rgb<u8>, Rgb<u8>)) {
    match button_id {
        0 => ("ACCEPT", (GREEN, BRIGHT_GREEN)),
        1 => ("REJECT", (RED, BRIGHT_RED)),
        2 => ("STOP", (RED, BRIGHT_RED)),
        3 => ("RETRY", (GRAY, BRIGHT_GRAY)),
        4 => ("REWIND", (BLUE, BRIGHT_BLUE)),
        5 => ("TRUST", (GREEN, BRIGHT_GREEN)),
        6 => ("TAB", (BLUE, BRIGHT_BLUE)),
        7 => ("MIC", (PURPLE, BRIGHT_PURPLE)),
        8 => ("ENTER", (BLUE, BRIGHT_BLUE)),
        9 => ("CLEAR", (GRAY, BRIGHT_GRAY)),
        _ => ("?", (GRAY, BRIGHT_GRAY)),
    }
}
