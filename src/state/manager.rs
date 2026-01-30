use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Available models for the model selector
pub const MODELS: &[&str] = &["opus", "sonnet", "haiku"];

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
    /// Progress percentage (0-100)
    pub progress: u8,
    /// Current model name
    pub model: String,
    /// Index in MODELS array
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
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            task_name: "READY".to_string(),
            progress: 0,
            model: "opus".to_string(),
            model_index: 0,
            model_selecting: false,
            waiting_for_input: false,
            input_type: None,
            yolo_mode: false,
            connected: false,
            dictation_active: false,
            button_flash: None,
        }
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
        self.model_selecting = true;

        if direction > 0 {
            self.model_index = (self.model_index + 1) % MODELS.len();
        } else {
            self.model_index = self.model_index.checked_sub(1).unwrap_or(MODELS.len() - 1);
        }

        self.model = MODELS[self.model_index].to_string();
    }

    /// Confirm model selection (called when encoder is pressed)
    pub fn confirm_model(&mut self) {
        self.model_selecting = false;
    }

    /// Set model by name
    pub fn set_model(&mut self, model: &str) {
        if let Some(index) = MODELS.iter().position(|&m| m == model) {
            self.model_index = index;
            self.model = model.to_string();
        }
    }

    /// Reset to initial state
    pub fn reset(&mut self) {
        self.task_name = "READY".to_string();
        self.progress = 0;
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
}
