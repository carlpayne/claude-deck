use enigo::{Enigo, Key as EnigoKey, Keyboard, Settings};
use std::time::Duration;
use tracing::debug;

/// Key types for input
#[derive(Debug, Clone)]
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
    Space,
    // Function keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    // Character key (letter, number, or symbol)
    Char(char),
}

/// Parsed keyboard shortcut with modifiers
#[derive(Debug, Clone)]
pub struct KeyboardShortcut {
    pub cmd: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub key: Key,
}

impl KeyboardShortcut {
    /// Parse a shortcut string like "Cmd+Shift+C" or just "Enter"
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('+').collect();
        if parts.is_empty() {
            return None;
        }

        let mut shortcut = KeyboardShortcut {
            cmd: false,
            ctrl: false,
            alt: false,
            shift: false,
            key: Key::Enter,
        };

        for (i, part) in parts.iter().enumerate() {
            let part_lower = part.to_lowercase();
            let is_last = i == parts.len() - 1;

            if !is_last {
                // This is a modifier
                match part_lower.as_str() {
                    "cmd" | "command" | "meta" => shortcut.cmd = true,
                    "ctrl" | "control" => shortcut.ctrl = true,
                    "alt" | "option" | "opt" => shortcut.alt = true,
                    "shift" => shortcut.shift = true,
                    _ => return None, // Unknown modifier
                }
            } else {
                // This is the main key
                shortcut.key = string_to_key(part)?;
            }
        }

        Some(shortcut)
    }

    /// Check if this shortcut has any modifiers
    pub fn has_modifiers(&self) -> bool {
        self.cmd || self.ctrl || self.alt || self.shift
    }
}

/// Convert string to Key enum
pub fn string_to_key(s: &str) -> Option<Key> {
    let lower = s.to_lowercase();
    match lower.as_str() {
        "enter" | "return" => Some(Key::Enter),
        "escape" | "esc" => Some(Key::Escape),
        "tab" => Some(Key::Tab),
        "space" => Some(Key::Space),
        "up" => Some(Key::Up),
        "down" => Some(Key::Down),
        "left" => Some(Key::Left),
        "right" => Some(Key::Right),
        "pageup" => Some(Key::PageUp),
        "pagedown" => Some(Key::PageDown),
        "home" => Some(Key::Home),
        "end" => Some(Key::End),
        "backspace" => Some(Key::Backspace),
        "delete" => Some(Key::Delete),
        // Function keys
        "f1" => Some(Key::F1),
        "f2" => Some(Key::F2),
        "f3" => Some(Key::F3),
        "f4" => Some(Key::F4),
        "f5" => Some(Key::F5),
        "f6" => Some(Key::F6),
        "f7" => Some(Key::F7),
        "f8" => Some(Key::F8),
        "f9" => Some(Key::F9),
        "f10" => Some(Key::F10),
        "f11" => Some(Key::F11),
        "f12" => Some(Key::F12),
        // Single character (letter, number, symbol)
        _ if s.len() == 1 => Some(Key::Char(s.chars().next().unwrap())),
        _ => None,
    }
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
    pub fn send_key(&mut self, key: &Key) {
        let enigo_key = key_to_enigo(key);
        debug!("Sending key: {:?}", enigo_key);
        let _ = self.enigo.key(enigo_key, enigo::Direction::Click);
    }

    /// Send a keyboard shortcut (key with optional modifiers)
    pub fn send_shortcut(&mut self, shortcut: &KeyboardShortcut) {
        debug!("Sending shortcut: {:?}", shortcut);

        // First, ensure all modifiers are released (clean slate)
        // This helps when previous shortcuts may have left modifier state
        self.release_all_modifiers();

        // Build list of modifiers to press
        let mut modifiers = Vec::new();
        if shortcut.cmd {
            modifiers.push(EnigoKey::Meta);
        }
        if shortcut.ctrl {
            modifiers.push(EnigoKey::Control);
        }
        if shortcut.alt {
            modifiers.push(EnigoKey::Alt);
        }
        if shortcut.shift {
            modifiers.push(EnigoKey::Shift);
        }

        let main_key = key_to_enigo(&shortcut.key);
        self.send_key_with_modifiers(&modifiers, main_key);
    }

    /// Release all modifier keys to ensure clean state
    fn release_all_modifiers(&mut self) {
        let _ = self.enigo.key(EnigoKey::Meta, enigo::Direction::Release);
        let _ = self.enigo.key(EnigoKey::Control, enigo::Direction::Release);
        let _ = self.enigo.key(EnigoKey::Alt, enigo::Direction::Release);
        let _ = self.enigo.key(EnigoKey::Shift, enigo::Direction::Release);
        let _ = self.enigo.key(EnigoKey::RCommand, enigo::Direction::Release);
        let _ = self.enigo.key(EnigoKey::RControl, enigo::Direction::Release);
    }

    /// Parse and send a shortcut string like "Cmd+C" or "Enter"
    pub fn send_shortcut_string(&mut self, shortcut_str: &str) -> bool {
        if let Some(shortcut) = KeyboardShortcut::parse(shortcut_str) {
            self.send_shortcut(&shortcut);
            true
        } else {
            debug!("Failed to parse shortcut: {}", shortcut_str);
            false
        }
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

        // Small delay to ensure modifiers are registered
        std::thread::sleep(Duration::from_millis(10));

        // Press and release the main key
        let _ = self.enigo.key(key, enigo::Direction::Click);

        // Small delay before releasing modifiers
        std::thread::sleep(Duration::from_millis(10));

        // Release modifiers in reverse order
        for modifier in modifiers.iter().rev() {
            let _ = self.enigo.key(*modifier, enigo::Direction::Release);
        }

        // Delay after releasing to ensure system processes the release
        // before any subsequent keystrokes
        std::thread::sleep(Duration::from_millis(20));
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

/// Convert our Key enum to Enigo's key type
fn key_to_enigo(key: &Key) -> EnigoKey {
    match key {
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
        Key::Space => EnigoKey::Space,
        Key::F1 => EnigoKey::F1,
        Key::F2 => EnigoKey::F2,
        Key::F3 => EnigoKey::F3,
        Key::F4 => EnigoKey::F4,
        Key::F5 => EnigoKey::F5,
        Key::F6 => EnigoKey::F6,
        Key::F7 => EnigoKey::F7,
        Key::F8 => EnigoKey::F8,
        Key::F9 => EnigoKey::F9,
        Key::F10 => EnigoKey::F10,
        Key::F11 => EnigoKey::F11,
        Key::F12 => EnigoKey::F12,
        Key::Char(c) => EnigoKey::Unicode(*c),
    }
}
