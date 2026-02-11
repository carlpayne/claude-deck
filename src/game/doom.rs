use image::{Rgb, RgbImage};
use rusttype::Font;
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Instant;

use doomgeneric::input::keys;
use doomgeneric::input::KeyData;

use crate::device::{BUTTON_HEIGHT, BUTTON_WIDTH, STRIP_HEIGHT, STRIP_WIDTH};
use crate::display::renderer::{draw_text, text_width};

// Doom renders at 640x400, we scale to 560x352 (buttons 560x224 + strip 560x128)
// This gives ~1.59:1 aspect ratio, very close to Doom's native 1.6:1
const SCALED_WIDTH: usize = 560;
const SCALED_HEIGHT: usize = 352; // 224 (buttons) + 128 (strip)

// Auto-release timeout for sustained movement keys (ms)
const KEY_RELEASE_TIMEOUT_MS: u128 = 150;

const STRIP_BG: Rgb<u8> = Rgb([12, 14, 20]);

// --- Singleton doom thread state ---
// doomgeneric only supports a single init() call for the process lifetime.
// The doom thread + channels persist forever, DoomGame reconnects on each create.

struct DoomThreadState {
    frame_rx: Mutex<mpsc::Receiver<Vec<u8>>>,
    key_tx: Mutex<mpsc::Sender<KeyData>>,
}

static DOOM_THREAD: OnceLock<DoomThreadState> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoomState {
    WaitingForWad,
    Playing,
}

pub struct DoomGame {
    key_tx: mpsc::Sender<KeyData>,
    current_frame: Vec<u8>,
    dirty_buttons: [bool; 10],
    strip_dirty: bool,
    active_keys: HashMap<u8, Instant>,
    state: DoomState,
}

impl DoomGame {
    pub fn new() -> Self {
        let wad_dir = Self::config_dir();
        let wad_path = wad_dir.join("doom1.wad");

        if !wad_path.exists() {
            // No WAD file found - show instructions
            let (key_tx, _dummy_key_rx) = mpsc::channel();
            return Self {
                key_tx,
                current_frame: vec![0u8; SCALED_WIDTH * SCALED_HEIGHT * 3],
                dirty_buttons: [true; 10],
                strip_dirty: true,
                active_keys: HashMap::new(),
                state: DoomState::WaitingForWad,
            };
        }

        // Initialize doom thread exactly once (singleton)
        let thread_state = DOOM_THREAD.get_or_init(|| {
            let (frame_tx, frame_rx) = mpsc::sync_channel::<Vec<u8>>(1);
            let (key_tx, key_rx) = mpsc::channel::<KeyData>();

            let wad_dir = wad_dir.clone();
            thread::spawn(move || {
                // Set working directory to where the WAD lives
                if let Err(e) = std::env::set_current_dir(&wad_dir) {
                    eprintln!("Failed to set doom working directory: {}", e);
                    return;
                }

                let deck_doom = DeckDoom {
                    frame_tx,
                    key_rx,
                    scaled_buf: vec![0u8; SCALED_WIDTH * SCALED_HEIGHT * 3],
                    x_map: Vec::new(),
                    y_map: Vec::new(),
                };
                doomgeneric::game::init(deck_doom);
                loop {
                    doomgeneric::game::tick();
                }
            });

            DoomThreadState {
                frame_rx: Mutex::new(frame_rx),
                key_tx: Mutex::new(key_tx),
            }
        });

        // Clone a sender to talk to the persistent doom thread
        let key_tx = thread_state.key_tx.lock().unwrap().clone();

        // If doom is already running (re-entering), send Escape to open menu
        // so the user can start a new game via button presses (Enter)
        if DOOM_THREAD.get().is_some() {
            let _ = key_tx.send(KeyData {
                pressed: true,
                key: keys::KEY_ESCAPE,
            });
            let _ = key_tx.send(KeyData {
                pressed: false,
                key: keys::KEY_ESCAPE,
            });
        }

        Self {
            key_tx,
            current_frame: vec![0u8; SCALED_WIDTH * SCALED_HEIGHT * 3],
            dirty_buttons: [true; 10],
            strip_dirty: true,
            active_keys: HashMap::new(),
            state: DoomState::Playing,
        }
    }

    fn config_dir() -> std::path::PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        std::path::PathBuf::from(home).join(".config/claude-deck")
    }

    pub fn tick(&mut self) {
        match self.state {
            DoomState::WaitingForWad => {
                // Nothing to do — user must cycle away and back after placing WAD
            }
            DoomState::Playing => {
                // Try to receive new frame from the singleton doom thread
                if let Some(thread_state) = DOOM_THREAD.get() {
                    if let Ok(rx) = thread_state.frame_rx.lock() {
                        // Drain to latest frame (skip stale frames)
                        let mut got_frame = false;
                        while let Ok(frame) = rx.try_recv() {
                            self.current_frame = frame;
                            got_frame = true;
                        }
                        if got_frame {
                            self.dirty_buttons = [true; 10];
                            self.strip_dirty = true;
                        }
                    }
                }

                // Auto-release held keys after timeout
                let now = Instant::now();
                let expired: Vec<u8> = self
                    .active_keys
                    .iter()
                    .filter(|(_, &time)| {
                        now.duration_since(time).as_millis() >= KEY_RELEASE_TIMEOUT_MS
                    })
                    .map(|(&key, _)| key)
                    .collect();

                for key in expired {
                    self.active_keys.remove(&key);
                    let _ = self.key_tx.send(KeyData {
                        pressed: false,
                        key,
                    });
                }
            }
        }
    }

    pub fn render_button(&self, button_id: u8) -> RgbImage {
        let bid = button_id as usize;
        if bid >= 10 {
            return RgbImage::new(BUTTON_WIDTH, BUTTON_HEIGHT);
        }

        let mut img = RgbImage::new(BUTTON_WIDTH, BUTTON_HEIGHT);

        match self.state {
            DoomState::WaitingForWad => {
                // Dark red-tinted background
                for pixel in img.pixels_mut() {
                    *pixel = Rgb([20, 8, 8]);
                }
            }
            DoomState::Playing => {
                // Slice 112x112 region from the scaled frame
                // Button layout: 5 columns x 2 rows
                let col = bid % 5;
                let row = bid / 5;
                let x0 = col * 112;
                let y0 = row * 112;

                for py in 0..112u32 {
                    for px in 0..112u32 {
                        let src_x = x0 + px as usize;
                        let src_y = y0 + py as usize;
                        if src_x < SCALED_WIDTH && src_y < SCALED_HEIGHT {
                            let idx = (src_y * SCALED_WIDTH + src_x) * 3;
                            if idx + 2 < self.current_frame.len() {
                                let r = self.current_frame[idx];
                                let g = self.current_frame[idx + 1];
                                let b = self.current_frame[idx + 2];
                                img.put_pixel(px, py, Rgb([r, g, b]));
                            }
                        }
                    }
                }
            }
        }

        img
    }

    pub fn render_strip(&self, font: &Font) -> RgbImage {
        let mut img = RgbImage::new(STRIP_WIDTH, STRIP_HEIGHT);
        for pixel in img.pixels_mut() {
            *pixel = STRIP_BG;
        }

        match self.state {
            DoomState::WaitingForWad => {
                // Title
                let title = "DOOM";
                let tw = text_width(font, title, 48.0);
                let tx = ((STRIP_WIDTH as i32 - tw) / 2).max(0);
                draw_text(&mut img, font, title, tx, 5, 48.0, Rgb([200, 50, 50]));

                // Instructions
                let msg = "Place doom1.wad in ~/.config/claude-deck/";
                let mw = text_width(font, msg, 16.0);
                let mx = ((STRIP_WIDTH as i32 - mw) / 2).max(0);
                draw_text(&mut img, font, msg, mx, 65, 16.0, Rgb([180, 180, 180]));

                let msg2 = "Press encoder 3 to cycle games";
                let mw2 = text_width(font, msg2, 14.0);
                let mx2 = ((STRIP_WIDTH as i32 - mw2) / 2).max(0);
                draw_text(&mut img, font, msg2, mx2, 90, 14.0, Rgb([100, 100, 120]));
            }
            DoomState::Playing => {
                // Stretch bottom 128 rows of doom frame (560px) to fill full strip (800px)
                let y_start = 224usize; // rows 224..352 go to strip

                for py in 0..STRIP_HEIGHT {
                    for px in 0..STRIP_WIDTH {
                        // Nearest-neighbor horizontal scale: 800 → 560
                        let src_x = px as usize * SCALED_WIDTH / STRIP_WIDTH as usize;
                        let src_y = y_start + py as usize;
                        if src_y < SCALED_HEIGHT && src_x < SCALED_WIDTH {
                            let idx = (src_y * SCALED_WIDTH + src_x) * 3;
                            if idx + 2 < self.current_frame.len() {
                                let r = self.current_frame[idx];
                                let g = self.current_frame[idx + 1];
                                let b = self.current_frame[idx + 2];
                                img.put_pixel(px, py, Rgb([r, g, b]));
                            }
                        }
                    }
                }
            }
        }

        img
    }

    pub fn handle_encoder_rotate(&mut self, encoder: u8, direction: i8) {
        if self.state != DoomState::Playing {
            return;
        }

        let key = match (encoder, direction > 0) {
            (0, true) => *keys::KEY_UP,
            (0, false) => *keys::KEY_DOWN,
            (1, true) => *keys::KEY_STRAFERIGHT,
            (1, false) => *keys::KEY_STRAFELEFT,
            (2, true) => *keys::KEY_RIGHT,
            (2, false) => *keys::KEY_LEFT,
            _ => return,
        };

        // Send key-down if not already held, and reset auto-release timer
        if !self.active_keys.contains_key(&key) {
            let _ = self.key_tx.send(KeyData {
                pressed: true,
                key,
            });
        }
        // Reset/extend the held duration on each encoder tick
        self.active_keys.insert(key, Instant::now());
    }

    pub fn handle_encoder_press(&mut self, encoder: u8) {
        if self.state != DoomState::Playing {
            return;
        }

        // Encoder 0/1/2 click = fire (sustained hold, auto-released by tick())
        if encoder < 3 {
            let key = *keys::KEY_FIRE;
            if !self.active_keys.contains_key(&key) {
                let _ = self.key_tx.send(KeyData {
                    pressed: true,
                    key,
                });
            }
            self.active_keys.insert(key, Instant::now());
        }
    }

    pub fn handle_button_press(&mut self, _button_id: u8) {
        if self.state != DoomState::Playing {
            return;
        }

        // Button press = use (open doors/switches) + enter (menu confirm)
        let _ = self.key_tx.send(KeyData {
            pressed: true,
            key: *keys::KEY_USE,
        });
        let _ = self.key_tx.send(KeyData {
            pressed: false,
            key: *keys::KEY_USE,
        });
        let _ = self.key_tx.send(KeyData {
            pressed: true,
            key: keys::KEY_ENTER,
        });
        let _ = self.key_tx.send(KeyData {
            pressed: false,
            key: keys::KEY_ENTER,
        });
    }

    // --- Dirty tracking ---

    pub fn is_button_dirty(&self, button_id: u8) -> bool {
        if (button_id as usize) < 10 {
            self.dirty_buttons[button_id as usize]
        } else {
            false
        }
    }

    pub fn is_strip_dirty(&self) -> bool {
        self.strip_dirty
    }

    pub fn has_any_dirty(&self) -> bool {
        self.strip_dirty || self.dirty_buttons.iter().any(|&d| d)
    }

    pub fn clear_dirty(&mut self) {
        self.dirty_buttons = [false; 10];
        self.strip_dirty = false;
    }
}

// --- DeckDoom: implements doomgeneric::game::DoomGeneric trait ---

struct DeckDoom {
    frame_tx: mpsc::SyncSender<Vec<u8>>,
    key_rx: mpsc::Receiver<KeyData>,
    // Pre-allocated frame buffer (reused when channel is full)
    scaled_buf: Vec<u8>,
    // Pre-computed nearest-neighbor lookup tables (avoid per-pixel division)
    x_map: Vec<usize>,
    y_map: Vec<usize>, // stores row_offset = (src_y * xres) for direct indexing
}

impl doomgeneric::game::DoomGeneric for DeckDoom {
    fn draw_frame(&mut self, screen_buffer: &[u32], xres: usize, yres: usize) {
        // Build lookup tables on first call (resolution is fixed)
        if self.x_map.is_empty() {
            self.x_map = (0..SCALED_WIDTH)
                .map(|sx| sx * xres / SCALED_WIDTH)
                .collect();
            self.y_map = (0..SCALED_HEIGHT)
                .map(|sy| (sy * yres / SCALED_HEIGHT) * xres)
                .collect();
        }

        // Scale ARGB → RGB using pre-computed tables
        for sy in 0..SCALED_HEIGHT {
            let src_row = self.y_map[sy];
            let dst_row = sy * SCALED_WIDTH * 3;
            for sx in 0..SCALED_WIDTH {
                let argb = screen_buffer[src_row + self.x_map[sx]];
                let dst = dst_row + sx * 3;
                self.scaled_buf[dst] = ((argb >> 16) & 0xFF) as u8;
                self.scaled_buf[dst + 1] = ((argb >> 8) & 0xFF) as u8;
                self.scaled_buf[dst + 2] = (argb & 0xFF) as u8;
            }
        }

        // Send frame, reusing buffer if channel is full (avoids allocation)
        let buf = std::mem::replace(
            &mut self.scaled_buf,
            Vec::new(), // temporary empty placeholder
        );
        match self.frame_tx.try_send(buf) {
            Ok(()) => {
                // Frame consumed — allocate fresh buffer for next frame
                self.scaled_buf = vec![0u8; SCALED_WIDTH * SCALED_HEIGHT * 3];
            }
            Err(mpsc::TrySendError::Full(returned)) | Err(mpsc::TrySendError::Disconnected(returned)) => {
                // Channel full or dead — reuse the buffer (zero allocation)
                self.scaled_buf = returned;
            }
        }
    }

    fn get_key(&mut self) -> Option<KeyData> {
        self.key_rx.try_recv().ok()
    }

    fn set_window_title(&mut self, _title: &str) {
        // No-op for hardware device
    }
}
