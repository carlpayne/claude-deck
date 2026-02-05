pub mod config;
pub mod device;
pub mod display;
pub mod hooks;
pub mod input;
pub mod profiles;
pub mod state;
pub mod system;
pub mod web;

use anyhow::Result;
use std::sync::{Arc, RwLock as StdRwLock};
use tokio::sync::{mpsc, RwLock as TokioRwLock};
use tracing::{debug, error, info, warn};

use config::Config;
use device::{button_to_display_key, DeviceManager};
use display::DisplayRenderer;
use input::InputHandler;
use profiles::ProfileManager;
use state::AppState;

/// Command to refresh the display
#[derive(Debug)]
pub enum AppCommand {
    /// Redraw all buttons (e.g., after config change)
    RedrawButtons,
}

/// Main application struct
pub struct App {
    #[allow(dead_code)]
    config: Config,
    state: Arc<TokioRwLock<AppState>>,
    device: Option<DeviceManager>,
    display: DisplayRenderer,
    input: InputHandler,
    #[allow(dead_code)]
    profile_manager: Arc<StdRwLock<ProfileManager>>,
    /// Channel to receive commands (e.g., refresh from web UI)
    command_rx: mpsc::Receiver<AppCommand>,
}

impl App {
    /// Create a new application instance
    pub async fn new(
        config: Config,
        profile_manager: Arc<StdRwLock<ProfileManager>>,
        command_rx: mpsc::Receiver<AppCommand>,
    ) -> Result<Self> {
        let state = Arc::new(TokioRwLock::new(AppState::with_config(
            config.models.available.clone(),
            &config.models.default,
            config.new_session.terminal.clone(),
            config.device.brightness,
        )));

        // Try to connect to device
        let brightness = state.read().await.brightness;
        let device = match DeviceManager::connect().await {
            Ok(d) => {
                info!("Connected to device");

                // Wake up device with keep-alive and brightness
                if let Err(e) = d.keep_alive().await {
                    warn!("Keep-alive failed: {}", e);
                }
                if let Err(e) = d.set_brightness(brightness).await {
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

        let display = DisplayRenderer::new(&config, Arc::clone(&profile_manager))?;
        let input = InputHandler::new(state.clone(), Arc::clone(&profile_manager));

        Ok(Self {
            config,
            state,
            device,
            display,
            input,
            profile_manager,
            command_rx,
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

        let brightness = self.state.read().await.brightness;
        info!("Waking up device with brightness {}%...", brightness);
        device.set_brightness(brightness).await.ok();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Play startup animation
        self.play_startup_animation().await?;

        // Get state for rendering
        let state = self.state.read().await;

        // Render all buttons
        for button_id in 0..10u8 {
            let display_key = button_to_display_key(button_id);
            let image = self.display.render_button(button_id, false, &state)?;
            device.set_button_image(display_key, image).await?;
        }

        // Flush buttons first
        info!("Flushing button images...");
        device.flush().await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Render full LCD strip (800x128 continuous display)
        let strip_image = self.display.render_strip(&state)?;
        device.set_strip_image(strip_image).await?;
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

            let display_key = button_to_display_key(button_id);

            let image = self.display.render_solid_button(r, g, b)?;
            if device.set_button_image(display_key, image).await.is_err() {
                continue;
            }
            device.flush().await.ok();
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        // Brief pause at full rainbow
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Phase 2: Flash all buttons bright white
        for button_id in 0..10u8 {
            let display_key = button_to_display_key(button_id);
            let image = self.display.render_solid_button(255, 255, 255)?;
            device.set_button_image(display_key, image).await.ok();
        }
        device.flush().await.ok();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Phase 3: Fade to dark
        for brightness in (0..=10).rev() {
            let level = brightness * 25;
            for button_id in 0..10u8 {
                let display_key = button_to_display_key(button_id);
                let image = self.display.render_solid_button(level, level, level)?;
                device.set_button_image(display_key, image).await.ok();
            }
            device.flush().await.ok();
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
        let status_check_interval = std::time::Duration::from_millis(200);

        let mut last_app_check = std::time::Instant::now();
        let app_check_interval = std::time::Duration::from_millis(500);
        let mut pending_app_check: Option<tokio::task::JoinHandle<Option<String>>> = None;

        let mut last_lock_check = std::time::Instant::now();
        let lock_check_interval = std::time::Duration::from_secs(2); // Check every 2 seconds (security, not latency-critical)

        let mut last_gif_tick = std::time::Instant::now();
        let gif_tick_interval = std::time::Duration::from_millis(16); // 60 FPS tick rate

        // Track last device write to enforce cooldown (HID device needs time between operations)
        let mut last_device_write = std::time::Instant::now();
        let device_cooldown = std::time::Duration::from_millis(20); // Min gap between device operations

        loop {
            // Check for commands from web UI (non-blocking)
            while let Ok(cmd) = self.command_rx.try_recv() {
                match cmd {
                    AppCommand::RedrawButtons => {
                        info!("Received redraw command from web UI");
                        // Small delay to let any pending device operations complete
                        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                        if let Err(e) = self.redraw_all_buttons().await {
                            warn!("Failed to redraw buttons from web UI: {}", e);
                        }
                        last_device_write = std::time::Instant::now();
                    }
                }
            }
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
                // Skip input handling when screen is locked (security)
                let is_locked = self.state.read().await.screen_locked;
                if !is_locked {
                    if let Err(e) = self.input.handle_event(event).await {
                        warn!("Failed to handle input event: {}", e);
                    }
                    if let Err(e) = self.update_display().await {
                        debug!("Failed to update display: {}", e);
                    }
                    last_device_write = std::time::Instant::now();
                } else {
                    // Silently ignore input when locked
                    continue;
                }

                // Check if brightness was changed
                let brightness_changed = {
                    let mut state = self.state.write().await;
                    let changed = state.brightness_changed;
                    state.brightness_changed = false;
                    if changed {
                        Some(state.brightness)
                    } else {
                        None
                    }
                };
                if let Some(brightness) = brightness_changed {
                    if let Some(ref device) = self.device {
                        device.set_brightness(brightness).await.ok();
                    }
                }

                // Check if intro animation was requested
                let play_intro = {
                    let mut state = self.state.write().await;
                    let flag = state.play_intro;
                    state.play_intro = false;
                    flag
                };
                if play_intro {
                    self.play_startup_animation().await.ok();
                    if let Err(e) = self.redraw_all_buttons().await {
                        warn!("Failed to redraw buttons after intro: {}", e);
                    }
                    last_device_write = std::time::Instant::now();
                }
            } else if self.device.is_none() {
                // Try to reconnect periodically
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                if let Ok(d) = DeviceManager::connect().await {
                    info!("Reconnected to device");
                    self.device = Some(d);
                    self.state.write().await.connected = true;
                    if let Err(e) = self.render_initial_display().await {
                        warn!("Failed to render initial display on reconnect: {}", e);
                    }
                    last_device_write = std::time::Instant::now();
                }
            }

            // Check for pending long-press actions (hold-to-activate)
            match self.input.check_long_press().await {
                Ok(true) => {
                    if let Err(e) = self.update_display().await {
                        debug!("Failed to update display after long-press: {}", e);
                    }
                    last_device_write = std::time::Instant::now();
                }
                Err(e) => warn!("Failed to check long-press: {}", e),
                _ => {}
            }

            // Poll Claude Code status file periodically
            if last_status_check.elapsed() >= status_check_interval {
                last_status_check = std::time::Instant::now();
                match self.update_from_claude_status().await {
                    Ok(true) => {
                        if let Err(e) = self.update_display().await {
                            debug!("Failed to update display after status change: {}", e);
                        }
                        last_device_write = std::time::Instant::now();
                    }
                    Err(e) => debug!("Failed to update from Claude status: {}", e),
                    _ => {}
                }
            }

            // Poll focused app in background (osascript is slow ~144ms)
            // Check if previous background task completed
            if let Some(handle) = pending_app_check.take() {
                if handle.is_finished() {
                    if let Ok(Some(app)) = handle.await {
                        let mut state = self.state.write().await;
                        if state.focused_app != app {
                            info!("Focused app changed: '{}' -> '{}'", state.focused_app, app);
                            state.focused_app = app;
                            drop(state); // Release lock before redraw
                            if let Err(e) = self.redraw_all_buttons().await {
                                warn!("Failed to redraw buttons on app change: {}", e);
                            }
                            last_device_write = std::time::Instant::now();
                        }
                    }
                } else {
                    // Not finished yet, put it back
                    pending_app_check = Some(handle);
                }
            }

            // Spawn new background check if interval elapsed and no pending check
            if pending_app_check.is_none() && last_app_check.elapsed() >= app_check_interval {
                last_app_check = std::time::Instant::now();
                pending_app_check = Some(tokio::spawn(async {
                    system::get_focused_app().await
                }));
            }

            // Check if screen is locked (for security - disable input when locked)
            if last_lock_check.elapsed() >= lock_check_interval {
                last_lock_check = std::time::Instant::now();
                let is_locked = system::is_screen_locked().await;
                let was_locked = self.state.read().await.screen_locked;
                if is_locked != was_locked {
                    self.state.write().await.screen_locked = is_locked;
                    if is_locked {
                        info!("Screen locked - input disabled");
                    } else {
                        info!("Screen unlocked - input enabled");
                    }
                    // Update ALL buttons and strip to show locked/unlocked state
                    if let Err(e) = self.redraw_all_buttons().await {
                        warn!("Failed to redraw buttons for lock state: {}", e);
                    }
                    if let Err(e) = self.update_display().await {
                        warn!("Failed to update strip for lock state: {}", e);
                    }
                    last_device_write = std::time::Instant::now();
                }
            }

            // Update GIF animations (respect device cooldown to avoid HID conflicts)
            if last_gif_tick.elapsed() >= gif_tick_interval
                && last_device_write.elapsed() >= device_cooldown
            {
                last_gif_tick = std::time::Instant::now();
                if let Err(e) = self.update_gif_animations().await {
                    debug!("GIF animation update skipped (device busy): {}", e);
                } else {
                    last_device_write = std::time::Instant::now();
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        }
    }

    /// Update display based on state changes
    async fn update_display(&self) -> Result<()> {
        let device = match self.device.as_ref() {
            Some(d) => d,
            None => return Ok(()),
        };

        let state = self.state.read().await;

        // Update full LCD strip (800x128 continuous display)
        let strip_image = self.display.render_strip(&state)?;
        device.set_strip_image(strip_image).await?;

        // Update all MIC buttons (shows red when recording, flashes on long-press)
        for mic_button_id in self.find_mic_buttons(&state) {
            let display_key = button_to_display_key(mic_button_id);
            let mic_active = state.is_button_flashed(mic_button_id);
            let mic_button = self.display.render_button(mic_button_id, mic_active, &state)?;
            device.set_button_image(display_key, mic_button).await?;
        }

        device.flush().await?;

        Ok(())
    }

    /// Redraw all buttons (called when app profile changes)
    async fn redraw_all_buttons(&self) -> Result<()> {
        let device = match self.device.as_ref() {
            Some(d) => d,
            None => return Ok(()),
        };

        // Clear all GIF animations - new profile may have different GIFs or none
        {
            let animator = display::gif_animator();
            let lock_result = animator.lock();
            if let Ok(mut anim) = lock_result {
                anim.clear_all();
            }
        }

        let state = self.state.read().await;

        // Render all buttons with current profile
        for button_id in 0..10u8 {
            let display_key = button_to_display_key(button_id);
            let image = self.display.render_button(button_id, false, &state)?;
            device.set_button_image(display_key, image).await?;
        }

        device.flush().await?;

        // Spawn background tasks to load any pending GIFs (non-blocking)
        self.start_gif_background_loading();

        Ok(())
    }

    /// Start background loading for any GIFs that need to be fetched
    fn start_gif_background_loading(&self) {
        let animator = display::gif_animator();
        let urls_to_load = {
            let lock_result = animator.lock();
            match lock_result {
                Ok(mut anim) => {
                    let urls = anim.get_pending_urls();
                    // Mark them as loading to prevent duplicate loads
                    for url in &urls {
                        anim.mark_loading(url);
                    }
                    urls
                }
                Err(_) => return,
            }
        };

        // Spawn a background task for each URL
        for url in urls_to_load {
            let animator_clone = animator.clone();
            tokio::spawn(async move {
                info!("Loading GIF in background: {}", url);
                // Run the blocking fetch in a blocking task pool
                let url_clone = url.clone();
                let result =
                    tokio::task::spawn_blocking(move || display::gif::fetch_and_decode_gif(&url_clone))
                        .await;

                // Store result in cache
                let gif = result.ok().flatten();
                let lock_result = animator_clone.lock();
                if let Ok(mut anim) = lock_result {
                    if gif.is_some() {
                        info!("GIF loaded successfully: {}", url);
                    } else {
                        warn!("Failed to load GIF: {}", url);
                    }
                    anim.store_loaded_gif(url, gif);
                }
            });
        }
    }

    /// Find all button IDs that have a MIC action configured in the current profile
    fn find_mic_buttons(&self, state: &state::AppState) -> Vec<u8> {
        use profiles::ButtonAction;

        let manager = self.profile_manager.read().unwrap();
        let mut mic_buttons = Vec::new();
        if let Some(profile) = manager.find_profile_for_app(&state.focused_app) {
            for button in &profile.buttons {
                let config = button.to_button_config();
                if matches!(&config.action, ButtonAction::Custom(action) if *action == "MIC") {
                    mic_buttons.push(button.position);
                }
            }
        }
        mic_buttons
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

        // Even if no status file, check Claude settings for model changes
        if let Some(model) = Self::read_claude_settings_model().await {
            let mut state = self.state.write().await;
            if !state.model_selecting && state.model != model {
                state.set_model(&model);
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Read model directly from Claude Code settings.json
    async fn read_claude_settings_model() -> Option<String> {
        let home = std::env::var("HOME").ok()?;
        let settings_path = std::path::PathBuf::from(home).join(".claude/settings.json");

        let content = tokio::fs::read_to_string(&settings_path).await.ok()?;
        let json: serde_json::Value = serde_json::from_str(&content).ok()?;
        json.get("model")?.as_str().map(|s| s.to_string())
    }

    /// Update GIF animations and redraw changed buttons
    async fn update_gif_animations(&self) -> Result<()> {
        let device = match self.device.as_ref() {
            Some(d) => d,
            None => return Ok(()),
        };

        // Tick the animator and get buttons with their new frames
        let tick_results = {
            let animator = display::gif_animator();
            let lock_result = animator.lock();
            let results = match lock_result {
                Ok(mut anim) => anim.tick(),
                Err(_) => return Ok(()),
            };
            results
        };

        if tick_results.is_empty() {
            return Ok(());
        }

        // Update all dirty buttons
        let state = self.state.read().await;
        for result in tick_results {
            let display_key = button_to_display_key(result.button_id);
            let image = self
                .display
                .render_button_with_gif_frame(result.button_id, &state, &result.frame)?;
            device.set_button_image(display_key, image).await?;
        }
        device.flush().await?;

        Ok(())
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
