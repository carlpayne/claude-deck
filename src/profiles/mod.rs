//! App profiles for context-aware button configurations

pub mod store;

use image::Rgb;
use std::sync::{Arc, RwLock};

use crate::display::renderer::{
    BLUE, BRIGHT_BLUE, BRIGHT_GRAY, BRIGHT_GREEN, BRIGHT_PURPLE, BRIGHT_RED, GRAY, GREEN, ORANGE,
    PURPLE, RED,
};

use store::ProfileConfig;

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
    /// Send a keyboard shortcut (e.g., "Enter", "Cmd+C", "Ctrl+Shift+V")
    Key(String),
    /// Type text directly (with optional auto-submit)
    Text { value: String, auto_submit: bool },
    /// Emoji shortcode (types `:emoji:`) (with optional auto-submit)
    Emoji { value: String, auto_submit: bool },
    /// Custom action handled by the input handler
    Custom(&'static str),
}

/// Button configuration for rendering and actions
#[derive(Debug, Clone)]
pub struct ButtonConfig {
    pub label: &'static str,
    pub colors: (Rgb<u8>, Rgb<u8>),
    pub action: ButtonAction,
    /// Optional emoji character for button image
    pub emoji_image: Option<&'static str>,
    /// Optional custom image (base64 data URL)
    pub custom_image: Option<&'static str>,
    /// Optional GIF URL for animated button
    pub gif_url: Option<&'static str>,
}

/// Manager for profile configurations
/// Holds loaded profiles from config and provides lookup
#[derive(Debug, Clone, Default)]
pub struct ProfileManager {
    profiles: Vec<ProfileConfig>,
}

impl ProfileManager {
    /// Create a new profile manager with profiles from config
    pub fn new(profiles: Vec<ProfileConfig>) -> Self {
        Self { profiles }
    }

    /// Create a shared profile manager
    pub fn shared(profiles: Vec<ProfileConfig>) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self::new(profiles)))
    }

    /// Update profiles (e.g., after config reload)
    pub fn set_profiles(&mut self, profiles: Vec<ProfileConfig>) {
        self.profiles = profiles;
    }

    /// Get all profiles
    pub fn get_profiles(&self) -> &[ProfileConfig] {
        &self.profiles
    }

    /// Get a profile by name
    pub fn get_profile(&self, name: &str) -> Option<&ProfileConfig> {
        self.profiles.iter().find(|p| p.name == name)
    }

    /// Get a mutable profile by name
    pub fn get_profile_mut(&mut self, name: &str) -> Option<&mut ProfileConfig> {
        self.profiles.iter_mut().find(|p| p.name == name)
    }

    /// Find the profile that matches an application name
    pub fn find_profile_for_app(&self, app_name: &str) -> Option<&ProfileConfig> {
        // First check for specific app matches (non-wildcard)
        for profile in &self.profiles {
            if profile.match_apps.iter().any(|p| p != "*" && p.eq_ignore_ascii_case(app_name)) {
                return Some(profile);
            }
        }
        // Fall back to wildcard profile
        self.profiles.iter().find(|p| p.match_apps.contains(&"*".to_string()))
    }

    /// Get button config for an app, falling back to hardcoded defaults
    pub fn get_button_config(&self, app_name: &str, button_id: u8) -> ButtonConfig {
        // Try to find a matching profile with this button configured
        if let Some(profile) = self.find_profile_for_app(app_name) {
            if let Some(config) = profile.get_button(button_id) {
                return config;
            }
            // Profile exists but button not configured - return empty button
            // (don't fall back to hardcoded defaults)
            return ButtonConfig {
                label: "---",
                colors: (GRAY, BRIGHT_GRAY),
                action: ButtonAction::Custom(""),
                emoji_image: None,
                custom_image: None,
                gif_url: None,
            };
        }

        // No profile found at all - fall back to hardcoded defaults
        let profile = get_profile_for_app(app_name);
        profile.button_config(button_id)
    }
}

/// Get the appropriate profile for an application name
pub fn get_profile_for_app(app_name: &str) -> AppProfile {
    match app_name {
        "Slack" => AppProfile::Slack,
        _ => AppProfile::Claude,
    }
}

/// Slack button definition tuple type: (label, shortcode, colors, emoji)
/// emoji field should be the actual emoji character for Twemoji rendering
type SlackButtonDef = (&'static str, &'static str, (Rgb<u8>, Rgb<u8>), &'static str);

/// Slack button configurations - emoji shortcuts
const SLACK_BUTTONS: [SlackButtonDef; 10] = [
    // Position 0-4: Top row
    (
        "👍",
        ":+1:",
        (Rgb([255, 200, 50]), Rgb([255, 220, 100])),
        "👍",
    ),
    (
        "👎",
        ":-1:",
        (Rgb([100, 100, 120]), Rgb([130, 130, 150])),
        "👎",
    ),
    ("✅", ":white_check_mark:", (GREEN, BRIGHT_GREEN), "✅"),
    ("👀", ":eyes:", (PURPLE, BRIGHT_PURPLE), "👀"),
    (
        "🎉",
        ":tada:",
        (Rgb([220, 100, 180]), Rgb([255, 130, 210])),
        "🎉",
    ),
    // Position 5-9: Bottom row
    ("❤️", ":heart:", (RED, BRIGHT_RED), "❤️"),
    (
        "😂",
        ":joy:",
        (Rgb([255, 200, 50]), Rgb([255, 220, 100])),
        "😂",
    ),
    ("🔥", ":fire:", (ORANGE, Rgb([255, 180, 80])), "🔥"),
    ("💯", ":100:", (RED, BRIGHT_RED), "💯"),
    ("🙏", ":pray:", (BLUE, BRIGHT_BLUE), "🙏"),
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
                        action: ButtonAction::Emoji {
                            value: emoji.to_string(),
                            auto_submit: false,
                        },
                        emoji_image: Some(image),
                        custom_image: None,
                        gif_url: None,
                    }
                } else {
                    // Fallback for any unmapped buttons
                    ButtonConfig {
                        label: "?",
                        colors: (GRAY, BRIGHT_GRAY),
                        action: ButtonAction::Text {
                            value: "".to_string(),
                            auto_submit: false,
                        },
                        emoji_image: None,
                        custom_image: None,
                        gif_url: None,
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
                    custom_image: None,
                    gif_url: None,
                }
            }
        }
    }
}

/// Get Claude mode button configuration (label and colors)
pub fn claude_button_config(button_id: u8) -> (&'static str, (Rgb<u8>, Rgb<u8>)) {
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

/// Generate default profiles as ProfileConfig objects
pub fn generate_default_profiles() -> Vec<ProfileConfig> {
    use store::{ActionConfig, ButtonConfigEntry};

    let claude_buttons: Vec<ButtonConfigEntry> = (0..10)
        .map(|pos| {
            let (label, colors) = claude_button_config(pos);
            ButtonConfigEntry {
                position: pos,
                label: label.to_string(),
                color: store::rgb_to_hex(colors.0),
                bright_color: store::rgb_to_hex(colors.1),
                action: ActionConfig::Custom {
                    value: label.to_string(),
                },
                emoji_image: None,
                custom_image: None,
                gif_url: None,
            }
        })
        .collect();

    let slack_buttons: Vec<ButtonConfigEntry> = SLACK_BUTTONS
        .iter()
        .enumerate()
        .map(|(pos, (label, emoji, colors, image))| ButtonConfigEntry {
            position: pos as u8,
            label: label.to_string(),
            color: store::rgb_to_hex(colors.0),
            bright_color: store::rgb_to_hex(colors.1),
            action: ActionConfig::Emoji {
                value: emoji.to_string(),
                auto_submit: false,
            },
            emoji_image: Some(image.to_string()),
            custom_image: None,
            gif_url: None,
        })
        .collect();

    vec![
        ProfileConfig {
            name: "claude".to_string(),
            match_apps: vec!["*".to_string()],
            buttons: claude_buttons,
        },
        ProfileConfig {
            name: "slack".to_string(),
            match_apps: vec!["Slack".to_string()],
            buttons: slack_buttons,
        },
    ]
}
