//! Profile configuration types for serialization/deserialization
//!
//! These types are used to store button configurations in config.toml
//! and can be loaded/saved by the web UI.

use image::Rgb;
use serde::{Deserialize, Serialize};

use super::{ButtonAction, ButtonConfig};

/// Action configuration for buttons (serializable)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionConfig {
    /// Send a keyboard key
    Key { value: String },
    /// Type text directly
    Text {
        value: String,
        #[serde(default)]
        auto_submit: bool,
    },
    /// Emoji shortcode (types `:emoji:`)
    #[serde(alias = "slack_emoji")]  // Backwards compatibility
    Emoji {
        value: String,
        #[serde(default)]
        auto_submit: bool,
    },
    /// Custom action handled by the input handler
    Custom { value: String },
}

impl ActionConfig {
    /// Convert to runtime ButtonAction
    pub fn to_button_action(&self) -> ButtonAction {
        match self {
            ActionConfig::Key { value } => {
                // Store the shortcut string directly (e.g., "Enter", "Cmd+C")
                ButtonAction::Key(value.clone())
            }
            ActionConfig::Text { value, auto_submit } => ButtonAction::Text {
                value: value.clone(),
                auto_submit: *auto_submit,
            },
            ActionConfig::Emoji { value, auto_submit } => ButtonAction::Emoji {
                value: value.clone(),
                auto_submit: *auto_submit,
            },
            ActionConfig::Custom { value } => {
                // Custom actions use static strings, so we leak the string
                // This is acceptable since profiles are loaded once at startup
                ButtonAction::Custom(Box::leak(value.clone().into_boxed_str()))
            }
        }
    }

    /// Create from runtime ButtonAction
    pub fn from_button_action(action: &ButtonAction) -> Self {
        match action {
            ButtonAction::Key(shortcut) => ActionConfig::Key {
                value: shortcut.clone(),
            },
            ButtonAction::Text { value, auto_submit } => ActionConfig::Text {
                value: value.clone(),
                auto_submit: *auto_submit,
            },
            ButtonAction::Emoji { value, auto_submit } => ActionConfig::Emoji {
                value: value.clone(),
                auto_submit: *auto_submit,
            },
            ButtonAction::Custom(value) => ActionConfig::Custom {
                value: value.to_string(),
            },
        }
    }
}

/// Button configuration entry for a single button
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonConfigEntry {
    /// Button position (0-9)
    pub position: u8,
    /// Button label text
    pub label: String,
    /// Button color (hex string like "#00C864")
    pub color: String,
    /// Bright/active button color (hex string)
    pub bright_color: String,
    /// Action to perform when pressed
    pub action: ActionConfig,
    /// Optional emoji character for button image
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_image: Option<String>,
    /// Optional custom image (base64 data URL)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_image: Option<String>,
    /// Optional GIF URL for animated button
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gif_url: Option<String>,
}

impl ButtonConfigEntry {
    /// Convert to runtime ButtonConfig
    pub fn to_button_config(&self) -> ButtonConfig {
        let color = parse_hex_color(&self.color).unwrap_or(Rgb([80, 85, 95]));
        let bright_color = parse_hex_color(&self.bright_color).unwrap_or(Rgb([110, 115, 125]));

        ButtonConfig {
            label: Box::leak(self.label.clone().into_boxed_str()),
            colors: (color, bright_color),
            action: self.action.to_button_action(),
            emoji_image: self
                .emoji_image
                .as_ref()
                .map(|s| Box::leak(s.clone().into_boxed_str()) as &'static str),
            custom_image: self
                .custom_image
                .as_ref()
                .map(|s| Box::leak(s.clone().into_boxed_str()) as &'static str),
            gif_url: self
                .gif_url
                .as_ref()
                .map(|s| Box::leak(s.clone().into_boxed_str()) as &'static str),
        }
    }

    /// Create from runtime ButtonConfig with position
    pub fn from_button_config(position: u8, config: &ButtonConfig) -> Self {
        Self {
            position,
            label: config.label.to_string(),
            color: rgb_to_hex(config.colors.0),
            bright_color: rgb_to_hex(config.colors.1),
            action: ActionConfig::from_button_action(&config.action),
            emoji_image: config.emoji_image.map(|s| s.to_string()),
            custom_image: config.custom_image.map(|s| s.to_string()),
            gif_url: config.gif_url.map(|s| s.to_string()),
        }
    }
}

/// Profile configuration for an application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    /// Profile name (e.g., "claude", "slack")
    pub name: String,
    /// Applications this profile matches (e.g., ["Slack"], ["*"] for default)
    pub match_apps: Vec<String>,
    /// Button configurations
    pub buttons: Vec<ButtonConfigEntry>,
}

impl ProfileConfig {
    /// Check if this profile matches an application name
    pub fn matches_app(&self, app_name: &str) -> bool {
        self.match_apps.iter().any(|pattern| {
            if pattern == "*" {
                true
            } else {
                pattern.eq_ignore_ascii_case(app_name)
            }
        })
    }

    /// Get button config for a position, if defined
    pub fn get_button(&self, position: u8) -> Option<ButtonConfig> {
        self.buttons
            .iter()
            .find(|b| b.position == position)
            .map(|b| b.to_button_config())
    }
}

/// Parse a hex color string to Rgb
pub fn parse_hex_color(hex: &str) -> Option<Rgb<u8>> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

    Some(Rgb([r, g, b]))
}

/// Convert Rgb to hex string
pub fn rgb_to_hex(color: Rgb<u8>) -> String {
    format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2])
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color("#FF0000"), Some(Rgb([255, 0, 0])));
        assert_eq!(parse_hex_color("00FF00"), Some(Rgb([0, 255, 0])));
        assert_eq!(parse_hex_color("#0000FF"), Some(Rgb([0, 0, 255])));
        assert_eq!(parse_hex_color("invalid"), None);
    }

    #[test]
    fn test_rgb_to_hex() {
        assert_eq!(rgb_to_hex(Rgb([255, 0, 0])), "#FF0000");
        assert_eq!(rgb_to_hex(Rgb([0, 255, 0])), "#00FF00");
        assert_eq!(rgb_to_hex(Rgb([0, 0, 255])), "#0000FF");
    }

    #[test]
    fn test_profile_matches_app() {
        let profile = ProfileConfig {
            name: "test".to_string(),
            match_apps: vec!["Slack".to_string(), "Discord".to_string()],
            buttons: vec![],
        };

        assert!(profile.matches_app("Slack"));
        assert!(profile.matches_app("slack")); // Case insensitive
        assert!(profile.matches_app("Discord"));
        assert!(!profile.matches_app("Terminal"));
    }

    #[test]
    fn test_profile_wildcard() {
        let profile = ProfileConfig {
            name: "default".to_string(),
            match_apps: vec!["*".to_string()],
            buttons: vec![],
        };

        assert!(profile.matches_app("Slack"));
        assert!(profile.matches_app("Terminal"));
        assert!(profile.matches_app("Anything"));
    }
}
