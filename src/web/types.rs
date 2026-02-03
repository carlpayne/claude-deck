//! Request/response types for the web API

use serde::{Deserialize, Serialize};

use crate::profiles::store::{ActionConfig, ButtonConfigEntry, ProfileConfig};

/// Event emitted when configuration changes
#[derive(Debug, Clone)]
pub enum ConfigChangeEvent {
    /// A profile was updated
    ProfileUpdated(String),
    /// A specific button was updated
    ButtonUpdated { profile: String, position: u8 },
    /// Full config reload requested
    Reload,
}

/// Profile summary for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSummary {
    pub name: String,
    pub match_apps: Vec<String>,
    pub button_count: usize,
}

impl From<&ProfileConfig> for ProfileSummary {
    fn from(profile: &ProfileConfig) -> Self {
        Self {
            name: profile.name.clone(),
            match_apps: profile.match_apps.clone(),
            button_count: profile.buttons.len(),
        }
    }
}

/// Full profile response with all buttons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileResponse {
    pub name: String,
    pub match_apps: Vec<String>,
    pub buttons: Vec<ButtonConfigEntry>,
}

impl From<&ProfileConfig> for ProfileResponse {
    fn from(profile: &ProfileConfig) -> Self {
        Self {
            name: profile.name.clone(),
            match_apps: profile.match_apps.clone(),
            buttons: profile.buttons.clone(),
        }
    }
}

/// Request to update a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProfileRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_apps: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buttons: Option<Vec<ButtonConfigEntry>>,
}

/// Request to update a single button
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateButtonRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bright_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<ActionConfig>,
    /// Emoji image - empty string means "clear/remove", None means "don't change"
    #[serde(default)]
    pub emoji_image: Option<String>,
    /// Custom image (base64 data URL) - empty string means "clear/remove"
    #[serde(default)]
    pub custom_image: Option<String>,
    /// GIF URL - empty string means "clear/remove"
    #[serde(default)]
    pub gif_url: Option<String>,
}

/// Color preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorPreset {
    pub name: String,
    pub color: String,
    pub bright_color: String,
}

/// Available action types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionType {
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub action_type: String,
}

/// Available key for Key actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableKey {
    pub name: String,
    pub value: String,
}

/// Available built-in action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinAction {
    pub name: String,
    pub value: String,
    pub description: String,
}

/// Actions API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionsResponse {
    pub action_types: Vec<ActionType>,
    pub available_keys: Vec<AvailableKey>,
    pub modifier_keys: Vec<ModifierKey>,
    pub builtin_actions: Vec<BuiltinAction>,
}

/// Colors API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorsResponse {
    pub presets: Vec<ColorPreset>,
}

/// Generic API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

/// Get default color presets
pub fn get_color_presets() -> Vec<ColorPreset> {
    vec![
        // Row 1: Warm colors
        ColorPreset {
            name: "Red".to_string(),
            color: "#DC3C3C".to_string(),
            bright_color: "#FF5050".to_string(),
        },
        ColorPreset {
            name: "Orange".to_string(),
            color: "#DC8C32".to_string(),
            bright_color: "#FFB450".to_string(),
        },
        ColorPreset {
            name: "Yellow".to_string(),
            color: "#D4B000".to_string(),
            bright_color: "#F0D020".to_string(),
        },
        ColorPreset {
            name: "Lime".to_string(),
            color: "#78C800".to_string(),
            bright_color: "#96F020".to_string(),
        },
        // Row 2: Cool colors
        ColorPreset {
            name: "Green".to_string(),
            color: "#00C864".to_string(),
            bright_color: "#32DC82".to_string(),
        },
        ColorPreset {
            name: "Teal".to_string(),
            color: "#00A896".to_string(),
            bright_color: "#20D0BE".to_string(),
        },
        ColorPreset {
            name: "Cyan".to_string(),
            color: "#00A0C8".to_string(),
            bright_color: "#30C8F0".to_string(),
        },
        ColorPreset {
            name: "Blue".to_string(),
            color: "#3C78C8".to_string(),
            bright_color: "#5096F0".to_string(),
        },
        // Row 3: Purple/Pink
        ColorPreset {
            name: "Indigo".to_string(),
            color: "#5050C8".to_string(),
            bright_color: "#7070F0".to_string(),
        },
        ColorPreset {
            name: "Purple".to_string(),
            color: "#8C50C8".to_string(),
            bright_color: "#AA64F0".to_string(),
        },
        ColorPreset {
            name: "Pink".to_string(),
            color: "#DC50A0".to_string(),
            bright_color: "#FF70C0".to_string(),
        },
        ColorPreset {
            name: "Rose".to_string(),
            color: "#DC5070".to_string(),
            bright_color: "#FF7090".to_string(),
        },
        // Row 4: Neutrals
        ColorPreset {
            name: "Gray".to_string(),
            color: "#606878".to_string(),
            bright_color: "#808898".to_string(),
        },
        ColorPreset {
            name: "Dark".to_string(),
            color: "#384050".to_string(),
            bright_color: "#506070".to_string(),
        },
        ColorPreset {
            name: "Brown".to_string(),
            color: "#8B6040".to_string(),
            bright_color: "#A87850".to_string(),
        },
        ColorPreset {
            name: "White".to_string(),
            color: "#B0B8C0".to_string(),
            bright_color: "#D0D8E0".to_string(),
        },
    ]
}

/// Get available action types
pub fn get_action_types() -> Vec<ActionType> {
    vec![
        ActionType {
            name: "Custom".to_string(),
            description: "Built-in action handled by the application".to_string(),
            action_type: "custom".to_string(),
        },
        ActionType {
            name: "Key".to_string(),
            description: "Send a keyboard key".to_string(),
            action_type: "key".to_string(),
        },
        ActionType {
            name: "Text".to_string(),
            description: "Type text directly".to_string(),
            action_type: "text".to_string(),
        },
        ActionType {
            name: "Emoji".to_string(),
            description: "Type an emoji shortcode (e.g. :+1:)".to_string(),
            action_type: "emoji".to_string(),
        },
    ]
}

/// Get available keys
pub fn get_available_keys() -> Vec<AvailableKey> {
    vec![
        // Navigation
        AvailableKey { name: "Enter".to_string(), value: "Enter".to_string() },
        AvailableKey { name: "Escape".to_string(), value: "Escape".to_string() },
        AvailableKey { name: "Tab".to_string(), value: "Tab".to_string() },
        AvailableKey { name: "Space".to_string(), value: "Space".to_string() },
        // Arrow keys
        AvailableKey { name: "Up".to_string(), value: "Up".to_string() },
        AvailableKey { name: "Down".to_string(), value: "Down".to_string() },
        AvailableKey { name: "Left".to_string(), value: "Left".to_string() },
        AvailableKey { name: "Right".to_string(), value: "Right".to_string() },
        // Page navigation
        AvailableKey { name: "Page Up".to_string(), value: "PageUp".to_string() },
        AvailableKey { name: "Page Down".to_string(), value: "PageDown".to_string() },
        AvailableKey { name: "Home".to_string(), value: "Home".to_string() },
        AvailableKey { name: "End".to_string(), value: "End".to_string() },
        // Editing
        AvailableKey { name: "Backspace".to_string(), value: "Backspace".to_string() },
        AvailableKey { name: "Delete".to_string(), value: "Delete".to_string() },
        // Function keys
        AvailableKey { name: "F1".to_string(), value: "F1".to_string() },
        AvailableKey { name: "F2".to_string(), value: "F2".to_string() },
        AvailableKey { name: "F3".to_string(), value: "F3".to_string() },
        AvailableKey { name: "F4".to_string(), value: "F4".to_string() },
        AvailableKey { name: "F5".to_string(), value: "F5".to_string() },
        AvailableKey { name: "F6".to_string(), value: "F6".to_string() },
        AvailableKey { name: "F7".to_string(), value: "F7".to_string() },
        AvailableKey { name: "F8".to_string(), value: "F8".to_string() },
        AvailableKey { name: "F9".to_string(), value: "F9".to_string() },
        AvailableKey { name: "F10".to_string(), value: "F10".to_string() },
        AvailableKey { name: "F11".to_string(), value: "F11".to_string() },
        AvailableKey { name: "F12".to_string(), value: "F12".to_string() },
        // Letters
        AvailableKey { name: "A".to_string(), value: "A".to_string() },
        AvailableKey { name: "B".to_string(), value: "B".to_string() },
        AvailableKey { name: "C".to_string(), value: "C".to_string() },
        AvailableKey { name: "D".to_string(), value: "D".to_string() },
        AvailableKey { name: "E".to_string(), value: "E".to_string() },
        AvailableKey { name: "F".to_string(), value: "F".to_string() },
        AvailableKey { name: "G".to_string(), value: "G".to_string() },
        AvailableKey { name: "H".to_string(), value: "H".to_string() },
        AvailableKey { name: "I".to_string(), value: "I".to_string() },
        AvailableKey { name: "J".to_string(), value: "J".to_string() },
        AvailableKey { name: "K".to_string(), value: "K".to_string() },
        AvailableKey { name: "L".to_string(), value: "L".to_string() },
        AvailableKey { name: "M".to_string(), value: "M".to_string() },
        AvailableKey { name: "N".to_string(), value: "N".to_string() },
        AvailableKey { name: "O".to_string(), value: "O".to_string() },
        AvailableKey { name: "P".to_string(), value: "P".to_string() },
        AvailableKey { name: "Q".to_string(), value: "Q".to_string() },
        AvailableKey { name: "R".to_string(), value: "R".to_string() },
        AvailableKey { name: "S".to_string(), value: "S".to_string() },
        AvailableKey { name: "T".to_string(), value: "T".to_string() },
        AvailableKey { name: "U".to_string(), value: "U".to_string() },
        AvailableKey { name: "V".to_string(), value: "V".to_string() },
        AvailableKey { name: "W".to_string(), value: "W".to_string() },
        AvailableKey { name: "X".to_string(), value: "X".to_string() },
        AvailableKey { name: "Y".to_string(), value: "Y".to_string() },
        AvailableKey { name: "Z".to_string(), value: "Z".to_string() },
        // Numbers
        AvailableKey { name: "0".to_string(), value: "0".to_string() },
        AvailableKey { name: "1".to_string(), value: "1".to_string() },
        AvailableKey { name: "2".to_string(), value: "2".to_string() },
        AvailableKey { name: "3".to_string(), value: "3".to_string() },
        AvailableKey { name: "4".to_string(), value: "4".to_string() },
        AvailableKey { name: "5".to_string(), value: "5".to_string() },
        AvailableKey { name: "6".to_string(), value: "6".to_string() },
        AvailableKey { name: "7".to_string(), value: "7".to_string() },
        AvailableKey { name: "8".to_string(), value: "8".to_string() },
        AvailableKey { name: "9".to_string(), value: "9".to_string() },
        // Symbols
        AvailableKey { name: "Minus (-)".to_string(), value: "-".to_string() },
        AvailableKey { name: "Plus (+)".to_string(), value: "+".to_string() },
        AvailableKey { name: "Equals (=)".to_string(), value: "=".to_string() },
        AvailableKey { name: "Bracket [".to_string(), value: "[".to_string() },
        AvailableKey { name: "Bracket ]".to_string(), value: "]".to_string() },
        AvailableKey { name: "Semicolon (;)".to_string(), value: ";".to_string() },
        AvailableKey { name: "Quote (')".to_string(), value: "'".to_string() },
        AvailableKey { name: "Comma (,)".to_string(), value: ",".to_string() },
        AvailableKey { name: "Period (.)".to_string(), value: ".".to_string() },
        AvailableKey { name: "Slash (/)".to_string(), value: "/".to_string() },
        AvailableKey { name: "Backslash (\\)".to_string(), value: "\\".to_string() },
        AvailableKey { name: "Backtick (`)".to_string(), value: "`".to_string() },
    ]
}

/// Available modifier keys
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifierKey {
    pub name: String,
    pub value: String,
}

/// Get available modifier keys
pub fn get_modifier_keys() -> Vec<ModifierKey> {
    vec![
        ModifierKey { name: "Cmd".to_string(), value: "Cmd".to_string() },
        ModifierKey { name: "Ctrl".to_string(), value: "Ctrl".to_string() },
        ModifierKey { name: "Alt/Option".to_string(), value: "Alt".to_string() },
        ModifierKey { name: "Shift".to_string(), value: "Shift".to_string() },
    ]
}

/// Get available built-in actions for Claude Code
pub fn get_builtin_actions() -> Vec<BuiltinAction> {
    vec![
        BuiltinAction {
            name: "None".to_string(),
            value: "".to_string(),
            description: "Button does nothing".to_string(),
        },
        BuiltinAction {
            name: "Accept".to_string(),
            value: "ACCEPT".to_string(),
            description: "Accept the current suggestion (y)".to_string(),
        },
        BuiltinAction {
            name: "Reject".to_string(),
            value: "REJECT".to_string(),
            description: "Reject the current suggestion (n)".to_string(),
        },
        BuiltinAction {
            name: "Stop".to_string(),
            value: "STOP".to_string(),
            description: "Stop/interrupt current operation (Escape)".to_string(),
        },
        BuiltinAction {
            name: "Retry".to_string(),
            value: "RETRY".to_string(),
            description: "Retry the last request".to_string(),
        },
        BuiltinAction {
            name: "Rewind".to_string(),
            value: "REWIND".to_string(),
            description: "Go back to previous state".to_string(),
        },
        BuiltinAction {
            name: "Trust".to_string(),
            value: "TRUST".to_string(),
            description: "Trust and allow operations".to_string(),
        },
        BuiltinAction {
            name: "Tab".to_string(),
            value: "TAB".to_string(),
            description: "Autocomplete (Tab key)".to_string(),
        },
        BuiltinAction {
            name: "Mic".to_string(),
            value: "MIC".to_string(),
            description: "Toggle voice input".to_string(),
        },
        BuiltinAction {
            name: "Enter".to_string(),
            value: "ENTER".to_string(),
            description: "Submit/confirm (Enter key)".to_string(),
        },
        BuiltinAction {
            name: "Clear".to_string(),
            value: "CLEAR".to_string(),
            description: "Clear the current input".to_string(),
        },
    ]
}

/// Response for checking if a profile has default button configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasDefaultsResponse {
    pub has_defaults: bool,
}

/// Installed macOS application info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledApp {
    pub name: String,
    pub bundle_id: Option<String>,
}

/// Response containing list of installed apps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppsResponse {
    pub apps: Vec<InstalledApp>,
}

/// Request to create a new profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProfileRequest {
    pub name: String,
    pub match_apps: Vec<String>,
    pub copy_from: Option<String>,
}

/// Request to swap two buttons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapButtonsRequest {
    pub position1: u8,
    pub position2: u8,
}

/// Giphy search query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiphySearchQuery {
    pub q: String,
    #[serde(default = "default_giphy_limit")]
    pub limit: u32,
}

fn default_giphy_limit() -> u32 {
    12
}

/// A single GIF from Giphy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiphyGif {
    pub id: String,
    pub title: String,
    /// Small preview URL (for grid display)
    pub preview_url: String,
    /// Full size URL (for button display)
    pub url: String,
    /// Width of the GIF
    pub width: u32,
    /// Height of the GIF
    pub height: u32,
}

/// Giphy search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiphySearchResponse {
    pub gifs: Vec<GiphyGif>,
}
