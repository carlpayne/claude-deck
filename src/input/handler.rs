use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::device::InputEvent;
use crate::state::AppState;

use super::keystrokes::{Key, KeystrokeSender};

const LONG_PRESS_DURATION: Duration = Duration::from_secs(2);

/// Buttons that support hold-to-activate (fire when threshold reached, not on release)
const HOLD_TO_ACTIVATE_BUTTONS: &[u8] = &[7]; // MIC (clear line)

/// Convert device button ID to logical button ID
fn device_to_logical_button(device_id: u8) -> Option<u8> {
    if device_id < 10 {
        Some(device_id)
    } else {
        None
    }
}

/// Handles input events from the device
pub struct InputHandler {
    state: Arc<RwLock<AppState>>,
    keystroke_sender: KeystrokeSender,
    button_press_times: HashMap<u8, Instant>,
    long_press_fired: HashSet<u8>,
    encoder_state: EncoderState,
    dictation_state: DictationState,
}

/// Tracks encoder state for model selection timeout
struct EncoderState {
    model_selection_start: Option<Instant>,
}

/// Tracks dictation state
struct DictationState {
    active: bool,
    first_use: bool,
}

impl InputHandler {
    pub fn new(state: Arc<RwLock<AppState>>) -> Self {
        Self {
            state,
            keystroke_sender: KeystrokeSender::new(),
            button_press_times: HashMap::new(),
            long_press_fired: HashSet::new(),
            encoder_state: EncoderState {
                model_selection_start: None,
            },
            dictation_state: DictationState {
                active: false,
                first_use: true,
            },
        }
    }

    /// Check for pending long-press actions and fire them immediately
    /// Call this periodically from the main loop
    pub async fn check_long_press(&mut self) -> Result<bool> {
        let mut action_fired = false;

        for &button in HOLD_TO_ACTIVATE_BUTTONS {
            // Skip if already fired for this press
            if self.long_press_fired.contains(&button) {
                continue;
            }

            // Check if button is being held long enough
            if let Some(press_time) = self.button_press_times.get(&button) {
                if press_time.elapsed() >= LONG_PRESS_DURATION {
                    // Fire the long-press action now
                    if button == 7 {
                        self.clear_current_line();
                        self.state.write().await.flash_button(button);
                        action_fired = true;
                    }
                    // Mark as fired so we don't fire again
                    self.long_press_fired.insert(button);
                }
            }
        }

        Ok(action_fired)
    }

    /// Handle an input event from the device
    pub async fn handle_event(&mut self, event: InputEvent) -> Result<()> {
        match event {
            InputEvent::ButtonDown(device_id) => {
                if let Some(button) = device_to_logical_button(device_id) {
                    self.button_press_times.insert(button, Instant::now());
                }
            }
            InputEvent::ButtonUp(device_id) => {
                if let Some(button) = device_to_logical_button(device_id) {
                    self.handle_button_up(button).await?;
                }
            }
            InputEvent::EncoderRotate { encoder, direction } => {
                self.handle_encoder_rotate(encoder, direction).await?;
            }
            InputEvent::EncoderPress(encoder) => {
                self.handle_encoder_press(encoder).await?;
            }
            InputEvent::EncoderRelease(_) => {
                // Currently no action on encoder release
            }
        }
        Ok(())
    }

    /// Handle button release (determines short vs long press)
    async fn handle_button_up(&mut self, button: u8) -> Result<()> {
        let press_duration = self
            .button_press_times
            .remove(&button)
            .map(|t| t.elapsed())
            .unwrap_or_default();

        // Check if this was a hold-to-activate button that already fired
        let already_fired = self.long_press_fired.remove(&button);
        if already_fired {
            debug!("Button {} released (long-press already fired)", button);
            return Ok(());
        }

        let is_long_press = press_duration >= LONG_PRESS_DURATION;

        debug!(
            "Button {} released (duration: {:?}, long_press: {})",
            button, press_duration, is_long_press
        );

        match (button, is_long_press) {
            // Top row - immediate actions
            (0, _) => self.send_accept().await?,
            (1, _) => self.send_reject().await?,
            (2, _) => self.send_stop(),
            (3, _) => self.send_retry(),
            (4, _) => self.send_rewind(),

            // Bottom row - with long-press variants
            (5, _) => self.send_trust(),
            (6, false) => self.send_tab(),
            (6, true) => self.open_new_session().await,
            // MIC: short press = voice input, long press = clear line (handled by check_long_press)
            (7, false) => self.trigger_voice_input().await,
            (8, _) => self.send_enter(),
            (9, _) => self.send_clear_command().await?,
            _ => {}
        }

        Ok(())
    }

    /// Handle encoder rotation
    async fn handle_encoder_rotate(&mut self, encoder: u8, direction: i8) -> Result<()> {
        debug!("Encoder {} rotated: {}", encoder, direction);

        match encoder {
            0 => self.scroll_output(direction),
            1 => self.cycle_model(direction).await,
            2 => self.navigate_history(direction),
            _ => {}
        }

        Ok(())
    }

    /// Handle encoder press
    async fn handle_encoder_press(&mut self, encoder: u8) -> Result<()> {
        debug!("Encoder {} pressed", encoder);

        match encoder {
            0 => {
                // Re-run last command
                self.send_key(Key::Up);
                std::thread::sleep(Duration::from_millis(50));
                self.send_key(Key::Enter);
            }
            1 => {
                // Confirm model selection
                self.confirm_model().await;
            }
            2 => {
                // Select current option (send Enter)
                info!("Encoder 2 press: selecting option");
                self.send_key(Key::Enter);
            }
            3 => {
                // Jump to bottom
                self.send_key(Key::End);
            }
            _ => {}
        }

        Ok(())
    }

    // === Helper methods ===

    fn send_text(&mut self, text: &str) {
        self.keystroke_sender.send_text(text);
    }

    fn send_key(&mut self, key: Key) {
        self.keystroke_sender.send_key(key);
    }

    // === Button actions ===

    async fn send_accept(&mut self) -> Result<()> {
        info!("ACCEPT: sending Enter (select Yes)");
        self.send_key(Key::Enter);
        self.state.write().await.waiting_for_input = false;
        Ok(())
    }

    async fn send_reject(&mut self) -> Result<()> {
        info!("REJECT: sending Escape (cancel)");
        self.send_key(Key::Escape);
        self.state.write().await.waiting_for_input = false;
        Ok(())
    }

    fn send_stop(&mut self) {
        info!("STOP: sending Escape");
        self.send_key(Key::Escape);
    }

    fn send_retry(&mut self) {
        info!("RETRY: sending Up + Enter");
        self.send_key(Key::Up);
        std::thread::sleep(Duration::from_millis(50));
        self.send_key(Key::Enter);
    }

    fn send_enter(&mut self) {
        debug!("ENTER: sending Enter");
        self.send_key(Key::Enter);
    }

    fn send_trust(&mut self) {
        info!("TRUST: sending '2' (select option 2: don't ask again)");
        self.send_text("2");
    }

    fn send_tab(&mut self) {
        debug!("TAB: sending Tab");
        self.send_key(Key::Tab);
    }

    fn send_rewind(&mut self) {
        info!("REWIND: sending double Escape");
        self.send_key(Key::Escape);
        std::thread::sleep(Duration::from_millis(100));
        self.send_key(Key::Escape);
    }

    fn clear_current_line(&mut self) {
        info!("CLEAR LINE: Ctrl+U (Unix line kill)");
        // Ctrl+U clears from cursor to beginning of line (Unix standard)
        self.keystroke_sender.send_ctrl_u();
    }

    async fn send_clear_command(&mut self) -> Result<()> {
        info!("CLEAR: sending /clear + Enter");
        self.send_text("/clear");
        self.send_key(Key::Enter);
        self.state.write().await.task_name = "READY".to_string();
        Ok(())
    }

    async fn open_new_session(&mut self) {
        info!("Opening new terminal session");

        #[cfg(target_os = "macos")]
        {
            let yolo = self.state.read().await.yolo_mode;
            let cmd = if yolo {
                "claude --dangerously-skip-permissions"
            } else {
                "claude"
            };

            let script = format!(
                r#"tell application "Terminal"
                    do script "{}"
                    activate
                end tell"#,
                cmd
            );

            // Spawn in a thread that waits for completion to prevent zombies
            std::thread::spawn(move || {
                let _ = std::process::Command::new("osascript")
                    .arg("-e")
                    .arg(&script)
                    .output(); // .output() waits for completion
            });
        }
    }

    async fn trigger_voice_input(&mut self) {
        info!("Toggling voice dictation");

        // First use needs a warmup - send toggle twice to prime enigo
        if self.dictation_state.first_use {
            debug!("First dictation use - warming up enigo");
            self.keystroke_sender.send_dictation_toggle();
            std::thread::sleep(std::time::Duration::from_millis(200));
            self.dictation_state.first_use = false;
        }
        self.keystroke_sender.send_dictation_toggle();

        // Toggle visual state
        self.dictation_state.active = !self.dictation_state.active;
        self.state.write().await.dictation_active = self.dictation_state.active;
        info!(
            "Dictation state: {}",
            if self.dictation_state.active {
                "ON"
            } else {
                "OFF"
            }
        );
    }

    // === Encoder actions ===

    fn navigate_history(&mut self, direction: i8) {
        let key = if direction > 0 { Key::Down } else { Key::Up };
        self.send_key(key);
    }

    fn scroll_output(&mut self, direction: i8) {
        let key = if direction > 0 {
            Key::PageDown
        } else {
            Key::PageUp
        };
        self.send_key(key);
    }

    #[allow(dead_code)]
    fn reset_zoom(&mut self) {
        self.keystroke_sender.reset_zoom();
    }

    async fn cycle_model(&mut self, direction: i8) {
        let mut state = self.state.write().await;
        state.cycle_model(direction);
        self.encoder_state.model_selection_start = Some(Instant::now());
    }

    async fn confirm_model(&mut self) {
        debug!("confirm_model: starting");
        let model = {
            let mut state = self.state.write().await;
            state.confirm_model();
            state.model.clone()
        };

        info!("Switching to model: {}", model);
        self.send_text(&format!("/model {}", model));
        self.send_key(Key::Enter);
        self.encoder_state.model_selection_start = None;
    }
}
