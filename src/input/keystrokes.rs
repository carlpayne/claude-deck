use enigo::{Enigo, Key as EnigoKey, Keyboard, Settings};
use std::time::Duration;
use tracing::debug;

/// Key types for input
#[derive(Debug, Clone, Copy)]
pub enum Key {
    Enter,
    Escape,
    Tab,
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,
    Backspace,
    Delete,
}

/// Sends keystrokes to the focused window (attach mode)
pub struct KeystrokeSender {
    enigo: Enigo,
}

impl KeystrokeSender {
    pub fn new() -> Self {
        let enigo = Enigo::new(&Settings::default()).expect("Failed to initialize Enigo");
        Self { enigo }
    }

    /// Send a single key press
    pub fn send_key(&mut self, key: Key) {
        let enigo_key = match key {
            Key::Enter => EnigoKey::Return,
            Key::Escape => EnigoKey::Escape,
            Key::Tab => EnigoKey::Tab,
            Key::Up => EnigoKey::UpArrow,
            Key::Down => EnigoKey::DownArrow,
            Key::Left => EnigoKey::LeftArrow,
            Key::Right => EnigoKey::RightArrow,
            Key::PageUp => EnigoKey::PageUp,
            Key::PageDown => EnigoKey::PageDown,
            Key::Home => EnigoKey::Home,
            Key::End => EnigoKey::End,
            Key::Backspace => EnigoKey::Backspace,
            Key::Delete => EnigoKey::Delete,
        };

        debug!("Sending key: {:?}", enigo_key);
        let _ = self.enigo.key(enigo_key, enigo::Direction::Click);
    }

    /// Send text as typed characters
    pub fn send_text(&mut self, text: &str) {
        debug!("Sending text: {}", text);
        let _ = self.enigo.text(text);
    }

    /// Send Shift+Tab
    pub fn send_shift_tab(&mut self) {
        debug!("Sending Shift+Tab");
        let _ = self.enigo.key(EnigoKey::Shift, enigo::Direction::Press);
        let _ = self.enigo.key(EnigoKey::Tab, enigo::Direction::Click);
        let _ = self.enigo.key(EnigoKey::Shift, enigo::Direction::Release);
    }

    /// Send Alt+M (Option+M on macOS) - Toggle permission modes
    pub fn send_alt_m(&mut self) {
        debug!("Sending Alt+M (toggle permission modes)");
        let _ = self.enigo.key(EnigoKey::Alt, enigo::Direction::Press);
        let _ = self
            .enigo
            .key(EnigoKey::Unicode('m'), enigo::Direction::Click);
        let _ = self.enigo.key(EnigoKey::Alt, enigo::Direction::Release);
    }

    /// Send Escape sequence for Alt+M (for terminals that use escape sequences)
    pub fn send_escape_m(&mut self) {
        debug!("Sending Escape+M (meta key sequence)");
        let _ = self.enigo.key(EnigoKey::Escape, enigo::Direction::Click);
        std::thread::sleep(Duration::from_millis(10));
        let _ = self
            .enigo
            .key(EnigoKey::Unicode('m'), enigo::Direction::Click);
    }

    /// Send a key with modifiers
    pub fn send_key_with_modifiers(&mut self, modifiers: &[EnigoKey], key: EnigoKey) {
        // Press modifiers
        for modifier in modifiers {
            let _ = self.enigo.key(*modifier, enigo::Direction::Press);
        }

        // Press and release the main key
        let _ = self.enigo.key(key, enigo::Direction::Click);

        // Release modifiers in reverse order
        for modifier in modifiers.iter().rev() {
            let _ = self.enigo.key(*modifier, enigo::Direction::Release);
        }
    }

    // === Zoom controls ===

    pub fn zoom_in(&mut self) {
        debug!("Zoom in: Cmd++");
        self.send_key_with_modifiers(&[EnigoKey::Meta], EnigoKey::Unicode('+'));
    }

    pub fn zoom_out(&mut self) {
        debug!("Zoom out: Cmd+-");
        self.send_key_with_modifiers(&[EnigoKey::Meta], EnigoKey::Unicode('-'));
    }

    pub fn reset_zoom(&mut self) {
        debug!("Reset zoom: Cmd+0");
        self.send_key_with_modifiers(&[EnigoKey::Meta], EnigoKey::Unicode('0'));
    }

    pub fn select_all(&mut self) {
        debug!("Select all: Cmd+A");
        self.send_key_with_modifiers(&[EnigoKey::Meta], EnigoKey::Unicode('a'));
    }

    /// Send Ctrl+U (Unix line kill - clears input line)
    pub fn send_ctrl_u(&mut self) {
        debug!("Sending Ctrl+U (line kill)");
        self.send_key_with_modifiers(&[EnigoKey::Control], EnigoKey::Unicode('u'));
    }

    /// Send Cmd+Z (Undo)
    pub fn send_undo(&mut self) {
        debug!("Sending Cmd+Z (undo)");
        self.send_key_with_modifiers(&[EnigoKey::Meta], EnigoKey::Unicode('z'));
    }

    // === Convenience methods ===

    pub fn send_accept(&mut self) {
        self.send_text("y");
        std::thread::sleep(Duration::from_millis(10));
        let _ = self.enigo.key(EnigoKey::Return, enigo::Direction::Click);
    }

    pub fn send_reject(&mut self) {
        self.send_text("n");
        std::thread::sleep(Duration::from_millis(10));
        let _ = self.enigo.key(EnigoKey::Return, enigo::Direction::Click);
    }

    pub fn send_stop(&mut self) {
        let _ = self.enigo.key(EnigoKey::Escape, enigo::Direction::Click);
    }

    pub fn send_retry(&mut self) {
        let _ = self.enigo.key(EnigoKey::UpArrow, enigo::Direction::Click);
        std::thread::sleep(Duration::from_millis(50));
        let _ = self.enigo.key(EnigoKey::Return, enigo::Direction::Click);
    }

    pub fn send_clear(&mut self) {
        self.send_text("/clear");
        let _ = self.enigo.key(EnigoKey::Return, enigo::Direction::Click);
    }

    pub fn send_rewind(&mut self) {
        let _ = self.enigo.key(EnigoKey::Escape, enigo::Direction::Click);
        std::thread::sleep(Duration::from_millis(100));
        let _ = self.enigo.key(EnigoKey::Escape, enigo::Direction::Click);
    }

    pub fn navigate_history(&mut self, direction: i8) {
        let key = if direction > 0 {
            EnigoKey::DownArrow
        } else {
            EnigoKey::UpArrow
        };
        let _ = self.enigo.key(key, enigo::Direction::Click);
    }

    pub fn scroll_output(&mut self, direction: i8) {
        let key = if direction > 0 {
            EnigoKey::PageDown
        } else {
            EnigoKey::PageUp
        };
        let _ = self.enigo.key(key, enigo::Direction::Click);
    }

    pub fn send_model_switch(&mut self, model: &str) {
        self.send_text(&format!("/model {}", model));
        let _ = self.enigo.key(EnigoKey::Return, enigo::Direction::Click);
    }

    /// Send double Right Command to trigger dictation
    pub fn send_dictation_toggle(&mut self) {
        debug!("Sending double Right Command for dictation");
        // RCommand is Right Command key
        let _ = self.enigo.key(EnigoKey::RCommand, enigo::Direction::Click);
        std::thread::sleep(Duration::from_millis(100));
        let _ = self.enigo.key(EnigoKey::RCommand, enigo::Direction::Click);
    }
}

impl Default for KeystrokeSender {
    fn default() -> Self {
        Self::new()
    }
}
