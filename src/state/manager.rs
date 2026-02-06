use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Default models for the model selector (used if config not provided)
pub const DEFAULT_MODELS: &[&str] = &["opus", "sonnet", "haiku"];

/// Type of input the system is waiting for
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputType {
    /// Yes/No prompt (y/n)
    YesNo,
    /// Press enter to continue
    Continue,
    /// Tool permission request
    Permission,
}

/// Application state shared across components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    // Claude Code state
    /// Current task/tool name displayed on strip
    pub task_name: String,
    /// Detail about current tool (file path, command preview, etc.)
    pub tool_detail: Option<String>,
    /// Current model name
    pub model: String,
    /// Index in available_models array
    pub model_index: usize,
    /// True when encoder is being rotated for model selection
    pub model_selecting: bool,
    /// True when Claude is waiting for user input
    pub waiting_for_input: bool,
    /// Type of input being waited for
    pub input_type: Option<InputType>,

    // App state
    /// YOLO mode enabled (--dangerously-skip-permissions)
    pub yolo_mode: bool,
    /// Device is connected
    pub connected: bool,
    /// Dictation/voice input is active
    pub dictation_active: bool,
    /// Button that was just activated (for visual feedback), with timestamp
    #[serde(skip)]
    pub button_flash: Option<(u8, Instant)>,
    /// Currently focused application name (e.g., "Slack", "Terminal", "Code")
    pub focused_app: String,
    /// Flag to trigger intro animation replay
    #[serde(skip)]
    pub play_intro: bool,
    /// Screen is locked - input disabled for security
    #[serde(skip)]
    pub screen_locked: bool,
    /// Flash toggle for waiting-for-input animation (alternates on/off)
    #[serde(skip)]
    pub waiting_flash_on: bool,

    // Configuration
    /// Available models (from config)
    #[serde(skip)]
    pub available_models: Vec<String>,
    /// Terminal app for new sessions (from config)
    #[serde(skip)]
    pub terminal_app: String,
    /// Device brightness (from config)
    #[serde(skip)]
    pub brightness: u8,
    /// Flag to indicate brightness needs to be applied to device
    #[serde(skip)]
    pub brightness_changed: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let default_models: Vec<String> = DEFAULT_MODELS.iter().map(|s| s.to_string()).collect();
        let default_model = default_models.first().cloned().unwrap_or_else(|| "opus".to_string());

        Self {
            task_name: "READY".to_string(),
            tool_detail: None,
            model: default_model,
            model_index: 0,
            model_selecting: false,
            waiting_for_input: false,
            input_type: None,
            yolo_mode: false,
            connected: false,
            dictation_active: false,
            button_flash: None,
            focused_app: String::new(),
            play_intro: false,
            screen_locked: false,
            waiting_flash_on: false,
            available_models: default_models,
            terminal_app: "Terminal".to_string(),
            brightness: 80,
            brightness_changed: false,
        }
    }

    /// Create state with configuration
    pub fn with_config(
        available_models: Vec<String>,
        default_model: &str,
        terminal_app: String,
        brightness: u8,
    ) -> Self {
        let model_index = available_models
            .iter()
            .position(|m| m == default_model)
            .unwrap_or(0);
        let model = available_models
            .get(model_index)
            .cloned()
            .unwrap_or_else(|| "opus".to_string());

        Self {
            task_name: "READY".to_string(),
            tool_detail: None,
            model,
            model_index,
            model_selecting: false,
            waiting_for_input: false,
            input_type: None,
            yolo_mode: false,
            connected: false,
            dictation_active: false,
            button_flash: None,
            focused_app: String::new(),
            play_intro: false,
            screen_locked: false,
            waiting_flash_on: false,
            available_models,
            terminal_app,
            brightness,
            brightness_changed: false,
        }
    }

    /// Adjust brightness by a delta (positive or negative)
    /// Returns the new brightness value
    pub fn adjust_brightness(&mut self, delta: i8) -> u8 {
        let step = 20i16; // 20% steps (device supports 5 levels: 20, 40, 60, 80, 100)
        let change = delta as i16 * step;
        let new_brightness = (self.brightness as i16 + change).clamp(20, 100) as u8;
        if new_brightness != self.brightness {
            self.brightness = new_brightness;
            self.brightness_changed = true;
        }
        self.brightness
    }

    /// Flash a button for visual feedback (shows as active briefly)
    pub fn flash_button(&mut self, button: u8) {
        self.button_flash = Some((button, Instant::now()));
    }

    /// Check if a button should show as flashed (within 300ms of activation)
    pub fn is_button_flashed(&self, button: u8) -> bool {
        if let Some((flashed_button, instant)) = self.button_flash {
            if flashed_button == button && instant.elapsed().as_millis() < 300 {
                return true;
            }
        }
        false
    }

    /// Cycle through available models
    pub fn cycle_model(&mut self, direction: i8) {
        if self.available_models.is_empty() {
            return;
        }

        self.model_selecting = true;

        let len = self.available_models.len();
        if direction > 0 {
            self.model_index = (self.model_index + 1) % len;
        } else {
            self.model_index = self.model_index.checked_sub(1).unwrap_or(len - 1);
        }

        self.model = self.available_models[self.model_index].clone();
    }

    /// Confirm model selection (called when encoder is pressed)
    pub fn confirm_model(&mut self) {
        self.model_selecting = false;
    }

    /// Set model by name
    pub fn set_model(&mut self, model: &str) {
        if let Some(index) = self.available_models.iter().position(|m| m == model) {
            self.model_index = index;
            self.model = model.to_string();
        }
    }

    /// Reset to initial state
    pub fn reset(&mut self) {
        self.task_name = "READY".to_string();
        self.tool_detail = None;
        self.waiting_for_input = false;
        self.input_type = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cycle_model_forward() {
        let mut state = AppState::new();
        // Default models are ["opus", "sonnet", "haiku"]
        assert_eq!(state.model, "opus");
        assert_eq!(state.model_index, 0);

        state.cycle_model(1);
        assert_eq!(state.model, "sonnet");
        assert_eq!(state.model_index, 1);

        state.cycle_model(1);
        assert_eq!(state.model, "haiku");
        assert_eq!(state.model_index, 2);

        state.cycle_model(1);
        assert_eq!(state.model, "opus");
        assert_eq!(state.model_index, 0);
    }

    #[test]
    fn test_cycle_model_backward() {
        let mut state = AppState::new();

        state.cycle_model(-1);
        assert_eq!(state.model, "haiku");
        assert_eq!(state.model_index, 2);

        state.cycle_model(-1);
        assert_eq!(state.model, "sonnet");
        assert_eq!(state.model_index, 1);
    }

    #[test]
    fn test_set_model() {
        let mut state = AppState::new();

        state.set_model("sonnet");
        assert_eq!(state.model, "sonnet");
        assert_eq!(state.model_index, 1);

        state.set_model("invalid");
        assert_eq!(state.model, "sonnet"); // Unchanged
    }

    #[test]
    fn test_with_config() {
        let models = vec!["model-a".to_string(), "model-b".to_string()];
        let state = AppState::with_config(models, "model-b", "iTerm".to_string(), 75);

        assert_eq!(state.model, "model-b");
        assert_eq!(state.model_index, 1);
        assert_eq!(state.terminal_app, "iTerm");
        assert_eq!(state.brightness, 75);
    }
}
