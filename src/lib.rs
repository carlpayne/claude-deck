pub mod config;
pub mod device;
pub mod display;
pub mod hooks;
pub mod input;
pub mod profiles;
pub mod state;
pub mod system;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use config::Config;
use device::DeviceManager;
use display::DisplayRenderer;
use input::InputHandler;
use state::AppState;

/// Main application struct
pub struct App {
    #[allow(dead_code)]
    config: Config,
    state: Arc<RwLock<AppState>>,
    device: Option<DeviceManager>,
    display: DisplayRenderer,
    input: InputHandler,
}

impl App {
    /// Create a new application instance
    pub async fn new(config: Config) -> Result<Self> {
        let state = Arc::new(RwLock::new(AppState::new()));

        // Try to connect to device
        let device = match DeviceManager::connect().await {
            Ok(d) => {
                info!("Connected to device");

                // Wake up device with keep-alive and brightness
                if let Err(e) = d.keep_alive().await {
                    warn!("Keep-alive failed: {}", e);
                }
                if let Err(e) = d.set_brightness(100).await {
                    warn!("Set brightness failed: {}", e);
                }

                state.write().await.connected = true;
                Some(d)
            }
            Err(e) => {
                error!("Failed to connect to device: {}", e);
                None
            }
        };

        let display = DisplayRenderer::new(&config)?;
        let input = InputHandler::new(state.clone());

        Ok(Self {
            config,
            state,
            device,
            display,
            input,
        })
    }

    /// Run the main application loop
    pub async fn run(&mut self) -> Result<()> {
        // Initialize display with default button images
        self.render_initial_display().await?;
        self.run_main_loop().await
    }

    /// Render initial display state
    async fn render_initial_display(&mut self) -> Result<()> {
        let device = match self.device.as_ref() {
            Some(d) => d,
            None => return Ok(()),
        };

        // Reset device to accept new images, then wake up
        info!("Resetting device for new session...");
        device.reset().await.ok();
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        info!("Waking up device...");
        device.set_brightness(100).await.ok();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Play startup animation
        self.play_startup_animation().await?;

        // Get state for rendering
        let state = self.state.read().await;

        // Render all buttons
        // N4 display mapping:
        // - Top row (row 0): display keys 10-14
        // - Bottom row (row 1): display keys 5-9
        // Our button layout:
        // - Buttons 0-4 (ACCEPT, REJECT, STOP, RETRY, REWIND) → top row → display keys 10-14
        // - Buttons 5-9 (YES ALL, TAB, MIC, ENTER, YOLO) → bottom row → display keys 5-9
        for button_id in 0..10u8 {
            let display_key = if button_id < 5 {
                button_id + 10 // 0-4 → 10-14 (top row)
            } else {
                button_id // 5-9 → 5-9 (bottom row)
            };
            let image = self.display.render_button(button_id, false, &state)?;
            device.set_button_image(display_key, &image).await?;
        }

        // Flush buttons first
        info!("Flushing button images...");
        device.flush().await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Render LCD strip as 4 individual status panels
        for strip_button_id in 0..4u8 {
            let strip_image = self.display.render_strip_button(strip_button_id, &state)?;
            device
                .set_strip_button_image(strip_button_id, &strip_image)
                .await?;
        }
        drop(state);

        info!("Flushing strip images...");
        device.flush().await?;

        info!("Initial display render complete");
        Ok(())
    }

    /// Play a startup animation on the device
    async fn play_startup_animation(&self) -> Result<()> {
        let device = match self.device.as_ref() {
            Some(d) => d,
            None => return Ok(()),
        };

        info!("Playing startup animation...");

        // Animation colors - rainbow wave
        let colors: [(u8, u8, u8); 6] = [
            (255, 50, 50),  // Red
            (255, 150, 50), // Orange
            (255, 255, 50), // Yellow
            (50, 255, 100), // Green
            (50, 150, 255), // Blue
            (150, 50, 255), // Purple
        ];

        // Wave animation: light up buttons in sequence
        // Button order for wave effect (left to right, top then bottom):
        // Top row: 0, 1, 2, 3, 4 (display keys 10-14)
        // Bottom row: 5, 6, 7, 8, 9 (display keys 5-9)
        let wave_order: [u8; 10] = [0, 5, 1, 6, 2, 7, 3, 8, 4, 9];

        // Phase 1: Wave sweep with rainbow colors
        for (i, &button_id) in wave_order.iter().enumerate() {
            let color_idx = i % colors.len();
            let (r, g, b) = colors[color_idx];

            let display_key = if button_id < 5 {
                button_id + 10
            } else {
                button_id
            };

            let image = self.display.render_solid_button(r, g, b)?;
            device.set_button_image(display_key, &image).await?;
            device.flush().await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        // Brief pause at full rainbow
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Phase 2: Flash all buttons bright white
        for button_id in 0..10u8 {
            let display_key = if button_id < 5 {
                button_id + 10
            } else {
                button_id
            };
            let image = self.display.render_solid_button(255, 255, 255)?;
            device.set_button_image(display_key, &image).await?;
        }
        device.flush().await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Phase 3: Fade to dark
        for brightness in (0..=10).rev() {
            let level = brightness * 25;
            for button_id in 0..10u8 {
                let display_key = if button_id < 5 {
                    button_id + 10
                } else {
                    button_id
                };
                let image = self.display.render_solid_button(level, level, level)?;
                device.set_button_image(display_key, &image).await?;
            }
            device.flush().await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        }

        info!("Startup animation complete");
        Ok(())
    }

    /// Run the main loop - handle device events and inject keystrokes
    async fn run_main_loop(&mut self) -> Result<()> {
        info!("Running - keystrokes will be sent to focused window");

        let mut last_keepalive = std::time::Instant::now();
        let keepalive_interval = std::time::Duration::from_secs(10);

        let mut last_status_check = std::time::Instant::now();
        let status_check_interval = std::time::Duration::from_millis(500);

        let mut last_app_check = std::time::Instant::now();
        let app_check_interval = std::time::Duration::from_millis(500);

        loop {
            // Handle device events
            let event = if let Some(ref mut device) = self.device {
                // Send periodic keep-alive to prevent device timeout
                if last_keepalive.elapsed() >= keepalive_interval {
                    if let Err(e) = device.keep_alive().await {
                        warn!("Keep-alive failed: {}", e);
                    }
                    last_keepalive = std::time::Instant::now();
                }

                match device.poll_event().await {
                    Ok(event) => event,
                    Err(e) => {
                        // Check if device disconnected
                        let error_str = format!("{}", e);
                        if error_str.contains("disconnected") || error_str.contains("Disconnected")
                        {
                            warn!("Device disconnected, will try to reconnect...");
                            self.device = None;
                            self.state.write().await.connected = false;
                        }
                        None
                    }
                }
            } else {
                None
            };

            if let Some(event) = event {
                self.input.handle_event(event).await?;
                self.update_display().await?;

                // Check if intro animation was requested
                let play_intro = {
                    let mut state = self.state.write().await;
                    let flag = state.play_intro;
                    state.play_intro = false;
                    flag
                };
                if play_intro {
                    self.play_startup_animation().await?;
                    self.redraw_all_buttons().await?;
                }
            } else if self.device.is_none() {
                // Try to reconnect periodically
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                if let Ok(d) = DeviceManager::connect().await {
                    info!("Reconnected to device");
                    self.device = Some(d);
                    self.state.write().await.connected = true;
                    self.render_initial_display().await?;
                }
            }

            // Check for pending long-press actions (hold-to-activate)
            if self.input.check_long_press().await? {
                self.update_display().await?;
            }

            // Poll Claude Code status file periodically
            if last_status_check.elapsed() >= status_check_interval {
                last_status_check = std::time::Instant::now();
                if self.update_from_claude_status().await? {
                    self.update_display().await?;
                }
            }

            // Poll focused app and redraw buttons on app change
            if last_app_check.elapsed() >= app_check_interval {
                last_app_check = std::time::Instant::now();
                match self.update_focused_app().await {
                    Ok(true) => {
                        info!("Redrawing all buttons for app change");
                        self.redraw_all_buttons().await?;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        warn!("Error checking focused app: {}", e);
                    }
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }

    /// Update display based on state changes
    async fn update_display(&self) -> Result<()> {
        let device = match self.device.as_ref() {
            Some(d) => d,
            None => return Ok(()),
        };

        let state = self.state.read().await;

        // Update LCD strip panels
        for strip_button_id in 0..4u8 {
            let strip_image = self.display.render_strip_button(strip_button_id, &state)?;
            device
                .set_strip_button_image(strip_button_id, &strip_image)
                .await?;
        }

        // Update MIC button (shows red when recording, flashes on long-press)
        // Button 7 (MIC) is on bottom row → display key 7
        let mic_active = state.is_button_flashed(7);
        let mic_button = self.display.render_button(7, mic_active, &state)?;
        device.set_button_image(7, &mic_button).await?;

        device.flush().await?;

        Ok(())
    }

    /// Update focused app from system
    /// Returns true if app changed (requiring button redraw)
    async fn update_focused_app(&self) -> Result<bool> {
        match system::get_focused_app().await {
            Some(app) => {
                let mut state = self.state.write().await;
                if state.focused_app != app {
                    info!("Focused app changed: '{}' -> '{}'", state.focused_app, app);
                    state.focused_app = app;
                    return Ok(true);
                }
            }
            None => {
                warn!("Failed to get focused app");
            }
        }
        Ok(false)
    }

    /// Redraw all buttons (called when app profile changes)
    async fn redraw_all_buttons(&self) -> Result<()> {
        let device = match self.device.as_ref() {
            Some(d) => d,
            None => return Ok(()),
        };

        let state = self.state.read().await;

        // Render all buttons with current profile
        for button_id in 0..10u8 {
            let display_key = if button_id < 5 {
                button_id + 10 // 0-4 → 10-14 (top row)
            } else {
                button_id // 5-9 → 5-9 (bottom row)
            };
            let image = self.display.render_button(button_id, false, &state)?;
            device.set_button_image(display_key, &image).await?;
        }

        device.flush().await?;
        Ok(())
    }

    /// Update state from Claude Code status file
    /// Returns true if state was updated
    async fn update_from_claude_status(&self) -> Result<bool> {
        if let Some(status) = hooks::read_status().await? {
            let mut state = self.state.write().await;

            let mut changed = false;

            // Update task name
            if !status.task.is_empty() && state.task_name != status.task {
                state.task_name = status.task;
                changed = true;
            }

            // Update tool detail
            if state.tool_detail != status.tool_detail {
                state.tool_detail = status.tool_detail;
                changed = true;
            }

            // Update waiting for input
            if state.waiting_for_input != status.waiting_for_input {
                state.waiting_for_input = status.waiting_for_input;
                // Convert string input_type to InputType enum
                state.input_type =
                    status
                        .input_type
                        .and_then(|s| match s.to_lowercase().as_str() {
                            "permission" => Some(state::InputType::Permission),
                            "yesno" | "yes_no" => Some(state::InputType::YesNo),
                            "continue" => Some(state::InputType::Continue),
                            _ => None,
                        });
                changed = true;
            }

            // Update model if provided (but not while user is selecting)
            if let Some(model) = status.model {
                if !state.model_selecting && state.model != model {
                    state.set_model(&model);
                    changed = true;
                }
            }

            return Ok(changed);
        }

        Ok(false)
    }

    /// Gracefully shutdown the application
    pub async fn shutdown(&mut self) {
        info!("Shutting down claude-deck...");

        // Drop the device to release HID connection
        if let Some(device) = self.device.take() {
            device.disconnect().await;
        }

        info!("Shutdown complete");
    }
}
