use enigo::{Enigo, Key as EnigoKey, Keyboard, Settings};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tracing::{debug, warn};

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

/// Commands executed on the dedicated keystroke thread
enum KeystrokeCmd {
    Key(Key),
    Text(String),
    Shortcut(KeyboardShortcut),
    ShortcutString(String),
    ShiftTab,
    AltM,
    EscapeM,
    ZoomIn,
    ZoomOut,
    ResetZoom,
    SelectAll,
    CtrlU,
    Undo,
    Retry,
    Clear,
    Rewind,
    NavigateHistory(i8),
    ScrollOutput(i8),
    ModelSwitch(String),
    DictationToggle,
    DictationWarmup,
}

/// Sends keystrokes to the focused window via a dedicated background thread.
///
/// All public methods are non-blocking — delays for multi-key sequences stay
/// off the async main loop so device I/O and animations keep running.
#[derive(Clone)]
pub struct KeystrokeSender {
    tx: mpsc::Sender<KeystrokeCmd>,
}

impl KeystrokeSender {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        thread::Builder::new()
            .name("keystroke-worker".into())
            .spawn(move || keystroke_worker(rx))
            .expect("Failed to start keystroke worker thread");
        Self { tx }
    }

    fn send(&self, cmd: KeystrokeCmd) {
        if let Err(e) = self.tx.send(cmd) {
            warn!("Keystroke worker unavailable: {}", e);
        }
    }

    /// Send a single key press
    pub fn send_key(&self, key: &Key) {
        self.send(KeystrokeCmd::Key(key.clone()));
    }

    /// Send a keyboard shortcut (key with optional modifiers)
    pub fn send_shortcut(&self, shortcut: &KeyboardShortcut) {
        self.send(KeystrokeCmd::Shortcut(shortcut.clone()));
    }

    /// Parse and send a shortcut string like "Cmd+C" or "Enter"
    pub fn send_shortcut_string(&self, shortcut_str: &str) -> bool {
        if KeyboardShortcut::parse(shortcut_str).is_some() {
            self.send(KeystrokeCmd::ShortcutString(shortcut_str.to_string()));
            true
        } else {
            debug!("Failed to parse shortcut: {}", shortcut_str);
            false
        }
    }

    /// Send text as typed characters
    pub fn send_text(&self, text: &str) {
        self.send(KeystrokeCmd::Text(text.to_string()));
    }

    /// Send Shift+Tab
    pub fn send_shift_tab(&self) {
        self.send(KeystrokeCmd::ShiftTab);
    }

    /// Send Alt+M (Option+M on macOS) - Toggle permission modes
    pub fn send_alt_m(&self) {
        self.send(KeystrokeCmd::AltM);
    }

    /// Send Escape sequence for Alt+M (for terminals that use escape sequences)
    pub fn send_escape_m(&self) {
        self.send(KeystrokeCmd::EscapeM);
    }

    // === Zoom controls ===

    pub fn zoom_in(&self) {
        self.send(KeystrokeCmd::ZoomIn);
    }

    pub fn zoom_out(&self) {
        self.send(KeystrokeCmd::ZoomOut);
    }

    pub fn reset_zoom(&self) {
        self.send(KeystrokeCmd::ResetZoom);
    }

    pub fn select_all(&self) {
        self.send(KeystrokeCmd::SelectAll);
    }

    /// Send Ctrl+U (Unix line kill - clears input line)
    pub fn send_ctrl_u(&self) {
        self.send(KeystrokeCmd::CtrlU);
    }

    /// Send Cmd+Z (Undo)
    pub fn send_undo(&self) {
        self.send(KeystrokeCmd::Undo);
    }

    // === Convenience methods ===

    pub fn send_accept(&self) {
        self.send_text("y");
        self.send_key(&Key::Enter);
    }

    pub fn send_reject(&self) {
        self.send_text("n");
        self.send_key(&Key::Enter);
    }

    pub fn send_stop(&self) {
        self.send_key(&Key::Escape);
    }

    pub fn send_retry(&self) {
        self.send(KeystrokeCmd::Retry);
    }

    pub fn send_clear(&self) {
        self.send(KeystrokeCmd::Clear);
    }

    pub fn send_rewind(&self) {
        self.send(KeystrokeCmd::Rewind);
    }

    pub fn navigate_history(&self, direction: i8) {
        self.send(KeystrokeCmd::NavigateHistory(direction));
    }

    pub fn scroll_output(&self, direction: i8) {
        self.send(KeystrokeCmd::ScrollOutput(direction));
    }

    pub fn send_model_switch(&self, model: &str) {
        self.send(KeystrokeCmd::ModelSwitch(model.to_string()));
    }

    /// Send double Right Command to trigger dictation
    pub fn send_dictation_toggle(&self) {
        self.send(KeystrokeCmd::DictationToggle);
    }

    /// Warm up enigo then toggle dictation (first-use path)
    pub fn send_dictation_warmup(&self) {
        self.send(KeystrokeCmd::DictationWarmup);
    }
}

impl Default for KeystrokeSender {
    fn default() -> Self {
        Self::new()
    }
}

fn keystroke_worker(rx: mpsc::Receiver<KeystrokeCmd>) {
    let mut enigo = Enigo::new(&Settings::default()).expect("Failed to initialize Enigo");

    while let Ok(cmd) = rx.recv() {
        execute(&mut enigo, cmd);
    }
}

fn execute(enigo: &mut Enigo, cmd: KeystrokeCmd) {
    match cmd {
        KeystrokeCmd::Key(key) => {
            let enigo_key = key_to_enigo(&key);
            debug!("Sending key: {:?}", enigo_key);
            let _ = enigo.key(enigo_key, enigo::Direction::Click);
        }
        KeystrokeCmd::Text(text) => {
            debug!("Sending text: {}", text);
            let _ = enigo.text(&text);
        }
        KeystrokeCmd::Shortcut(shortcut) => {
            send_shortcut(enigo, &shortcut);
        }
        KeystrokeCmd::ShortcutString(s) => {
            if let Some(shortcut) = KeyboardShortcut::parse(&s) {
                send_shortcut(enigo, &shortcut);
            }
        }
        KeystrokeCmd::ShiftTab => {
            debug!("Sending Shift+Tab");
            let _ = enigo.key(EnigoKey::Shift, enigo::Direction::Press);
            let _ = enigo.key(EnigoKey::Tab, enigo::Direction::Click);
            let _ = enigo.key(EnigoKey::Shift, enigo::Direction::Release);
        }
        KeystrokeCmd::AltM => {
            debug!("Sending Alt+M (toggle permission modes)");
            let _ = enigo.key(EnigoKey::Alt, enigo::Direction::Press);
            let _ = enigo.key(EnigoKey::Unicode('m'), enigo::Direction::Click);
            let _ = enigo.key(EnigoKey::Alt, enigo::Direction::Release);
        }
        KeystrokeCmd::EscapeM => {
            debug!("Sending Escape+M (meta key sequence)");
            let _ = enigo.key(EnigoKey::Escape, enigo::Direction::Click);
            thread::sleep(Duration::from_millis(10));
            let _ = enigo.key(EnigoKey::Unicode('m'), enigo::Direction::Click);
        }
        KeystrokeCmd::ZoomIn => {
            debug!("Zoom in: Cmd++");
            send_key_with_modifiers(enigo, &[EnigoKey::Meta], EnigoKey::Unicode('+'));
        }
        KeystrokeCmd::ZoomOut => {
            debug!("Zoom out: Cmd+-");
            send_key_with_modifiers(enigo, &[EnigoKey::Meta], EnigoKey::Unicode('-'));
        }
        KeystrokeCmd::ResetZoom => {
            debug!("Reset zoom: Cmd+0");
            send_key_with_modifiers(enigo, &[EnigoKey::Meta], EnigoKey::Unicode('0'));
        }
        KeystrokeCmd::SelectAll => {
            debug!("Select all: Cmd+A");
            send_key_with_modifiers(enigo, &[EnigoKey::Meta], EnigoKey::Unicode('a'));
        }
        KeystrokeCmd::CtrlU => {
            debug!("Sending Ctrl+U (line kill)");
            send_key_with_modifiers(enigo, &[EnigoKey::Control], EnigoKey::Unicode('u'));
        }
        KeystrokeCmd::Undo => {
            debug!("Sending Cmd+Z (undo)");
            send_key_with_modifiers(enigo, &[EnigoKey::Meta], EnigoKey::Unicode('z'));
        }
        KeystrokeCmd::Retry => {
            let _ = enigo.key(EnigoKey::UpArrow, enigo::Direction::Click);
            thread::sleep(Duration::from_millis(50));
            let _ = enigo.key(EnigoKey::Return, enigo::Direction::Click);
        }
        KeystrokeCmd::Clear => {
            let _ = enigo.text("/clear");
            let _ = enigo.key(EnigoKey::Return, enigo::Direction::Click);
        }
        KeystrokeCmd::Rewind => {
            let _ = enigo.key(EnigoKey::Escape, enigo::Direction::Click);
            thread::sleep(Duration::from_millis(100));
            let _ = enigo.key(EnigoKey::Escape, enigo::Direction::Click);
        }
        KeystrokeCmd::NavigateHistory(direction) => {
            let key = if direction > 0 {
                EnigoKey::DownArrow
            } else {
                EnigoKey::UpArrow
            };
            let _ = enigo.key(key, enigo::Direction::Click);
        }
        KeystrokeCmd::ScrollOutput(direction) => {
            let key = if direction > 0 {
                EnigoKey::PageDown
            } else {
                EnigoKey::PageUp
            };
            let _ = enigo.key(key, enigo::Direction::Click);
        }
        KeystrokeCmd::ModelSwitch(model) => {
            let _ = enigo.text(&format!("/model {}", model));
            thread::sleep(Duration::from_millis(150));
            let _ = enigo.key(EnigoKey::Return, enigo::Direction::Click);
        }
        KeystrokeCmd::DictationToggle => {
            send_dictation_toggle(enigo);
        }
        KeystrokeCmd::DictationWarmup => {
            debug!("First dictation use - warming up enigo");
            send_dictation_toggle(enigo);
            thread::sleep(Duration::from_millis(200));
            send_dictation_toggle(enigo);
        }
    }
}

fn send_shortcut(enigo: &mut Enigo, shortcut: &KeyboardShortcut) {
    debug!("Sending shortcut: {:?}", shortcut);

    // First, ensure all modifiers are released (clean slate)
    release_all_modifiers(enigo);

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
    send_key_with_modifiers(enigo, &modifiers, main_key);
}

fn release_all_modifiers(enigo: &mut Enigo) {
    let _ = enigo.key(EnigoKey::Meta, enigo::Direction::Release);
    let _ = enigo.key(EnigoKey::Control, enigo::Direction::Release);
    let _ = enigo.key(EnigoKey::Alt, enigo::Direction::Release);
    let _ = enigo.key(EnigoKey::Shift, enigo::Direction::Release);
    let _ = enigo.key(EnigoKey::RCommand, enigo::Direction::Release);
    let _ = enigo.key(EnigoKey::RControl, enigo::Direction::Release);
}

fn send_key_with_modifiers(enigo: &mut Enigo, modifiers: &[EnigoKey], key: EnigoKey) {
    for modifier in modifiers {
        let _ = enigo.key(*modifier, enigo::Direction::Press);
    }

    thread::sleep(Duration::from_millis(10));
    let _ = enigo.key(key, enigo::Direction::Click);
    thread::sleep(Duration::from_millis(10));

    for modifier in modifiers.iter().rev() {
        let _ = enigo.key(*modifier, enigo::Direction::Release);
    }

    thread::sleep(Duration::from_millis(20));
}

fn send_dictation_toggle(enigo: &mut Enigo) {
    debug!("Sending double Right Command for dictation");
    let _ = enigo.key(EnigoKey::RCommand, enigo::Direction::Click);
    thread::sleep(Duration::from_millis(100));
    let _ = enigo.key(EnigoKey::RCommand, enigo::Direction::Click);
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
