use anyhow::{Context, Result};
use clap::Parser;
use std::sync::{Arc, RwLock as StdRwLock};
use tokio::signal;
use tokio::sync::{mpsc, RwLock as TokioRwLock};
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use claude_deck::{
    config::Config,
    web::{self, ConfigChangeEvent},
    App, AppCommand,
};

#[derive(Parser, Debug)]
#[command(name = "claude-deck")]
#[command(about = "Hardware controller for Claude Code using AJAZZ AKP05E")]
#[command(version)]
struct Cli {
    /// Check device connection status and exit
    #[arg(long)]
    status: bool,

    /// Set device brightness (0-100)
    #[arg(long, value_name = "PERCENT", value_parser = clap::value_parser!(u8).range(0..=100))]
    brightness: Option<u8>,

    /// Install autostart on login (macOS LaunchAgent)
    #[arg(long)]
    install_autostart: bool,

    /// Uninstall autostart (remove LaunchAgent)
    #[arg(long)]
    uninstall_autostart: bool,

    /// Install Claude Code hooks for status integration
    #[arg(long)]
    install_hooks: bool,

    /// Uninstall Claude Code hooks
    #[arg(long)]
    uninstall_hooks: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // SAFETY: Setting SIGCHLD to SIG_IGN is async-signal-safe and prevents zombie
    // processes when spawning child commands (e.g., osascript for voice dictation).
    // We only ignore the signal rather than installing a custom handler, which is
    // the safest use of signal(). This is a well-established Unix pattern.
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGCHLD, libc::SIG_IGN);
    }

    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    // Handle simple commands first
    if cli.install_autostart {
        return install_autostart();
    }

    if cli.uninstall_autostart {
        return uninstall_autostart();
    }

    if cli.install_hooks {
        return install_hooks();
    }

    if cli.uninstall_hooks {
        return uninstall_hooks();
    }

    if cli.status {
        return check_status().await;
    }

    if let Some(brightness) = cli.brightness {
        return set_brightness(brightness).await;
    }

    // Load configuration
    let config = Config::load()?;

    info!("Starting claude-deck");

    // Initialize profile manager from config (uses std RwLock for sync access in renderer)
    let profile_manager = web::server::init_profile_manager(&config);
    let profile_manager = Arc::new(StdRwLock::new(profile_manager));
    let config = Arc::new(TokioRwLock::new(config));

    // Create config change channel
    let (change_tx, mut change_rx) = mpsc::channel::<ConfigChangeEvent>(16);

    // Create app command channel for triggering refreshes
    let (app_cmd_tx, app_cmd_rx) = mpsc::channel::<AppCommand>(16);

    // Create shared device state before web server so both can access it
    let config_snapshot = config.read().await.clone();
    let device_state = App::create_state(&config_snapshot);

    // Spawn web server if enabled
    let web_enabled = config.read().await.web.enabled;
    if web_enabled {
        let config_clone = Arc::clone(&config);
        let profile_manager_clone = Arc::clone(&profile_manager);
        let change_tx_clone = change_tx.clone();
        let device_state_clone = Arc::clone(&device_state);

        tokio::spawn(async move {
            if let Err(e) =
                web::start_server(config_clone, profile_manager_clone, change_tx_clone, device_state_clone).await
            {
                warn!("Web server error: {}", e);
            }
        });
    }

    // Spawn task to handle config change events and trigger display refreshes
    tokio::spawn(async move {
        while let Some(event) = change_rx.recv().await {
            info!("Config change event: {:?}", event);
            // Trigger display refresh for any config change
            if let Err(e) = app_cmd_tx.send(AppCommand::RedrawButtons).await {
                warn!("Failed to send redraw command: {}", e);
            }
        }
    });

    // Run the application with graceful shutdown
    let mut app = App::new(config_snapshot, Arc::clone(&profile_manager), app_cmd_rx, device_state).await?;

    // Set up signal handlers for graceful shutdown
    let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())?;

    let result = tokio::select! {
        result = app.run() => {
            result
        }
        _ = signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down...");
            Ok(())
        }
        _ = sigterm.recv() => {
            info!("Received SIGTERM, shutting down...");
            Ok(())
        }
    };

    // Always run shutdown
    app.shutdown().await;
    result
}

fn install_autostart() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        use std::fs;
        use std::path::PathBuf;

        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        let launch_agents = PathBuf::from(&home).join("Library/LaunchAgents");
        fs::create_dir_all(&launch_agents).context("Failed to create LaunchAgents directory")?;

        let plist_path = launch_agents.join("com.claude-deck.plist");
        let binary_path = std::env::current_exe().context("Failed to get current executable path")?;

        let plist_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.claude-deck</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{}/Library/Logs/claude-deck.log</string>
    <key>StandardErrorPath</key>
    <string>{}/Library/Logs/claude-deck.log</string>
</dict>
</plist>"#,
            binary_path.display(),
            home,
            home
        );

        fs::write(&plist_path, plist_content)
            .with_context(|| format!("Failed to write LaunchAgent plist to {:?}", plist_path))?;
        info!("Created LaunchAgent at {:?}", plist_path);
        info!("Run 'launchctl load {:?}' to start now", plist_path);
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("Autostart installation is only supported on macOS");
        Ok(())
    }
}

fn install_hooks() -> Result<()> {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    let home = std::env::var("HOME").context("HOME environment variable not set")?;

    // Claude Code hooks directory
    let hooks_dir = PathBuf::from(&home).join(".claude/hooks");
    fs::create_dir_all(&hooks_dir).context("Failed to create hooks directory")?;

    // Hook script content (embedded)
    let hook_script = include_str!("../hooks/claude-deck-hook.sh");
    let hook_path = hooks_dir.join("claude-deck-hook.sh");

    fs::write(&hook_path, hook_script)
        .with_context(|| format!("Failed to write hook script to {:?}", hook_path))?;

    // Make executable
    let mut perms = fs::metadata(&hook_path)
        .context("Failed to get hook script metadata")?
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&hook_path, perms).context("Failed to set hook script permissions")?;

    println!("✓ Installed hook script at {:?}", hook_path);

    // Update Claude Code settings
    let settings_dir = PathBuf::from(&home).join(".claude");
    fs::create_dir_all(&settings_dir).context("Failed to create .claude directory")?;

    let settings_path = settings_dir.join("settings.json");

    // Read existing settings or create new
    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)
            .with_context(|| format!("Failed to read settings from {:?}", settings_path))?;
        match serde_json::from_str(&content) {
            Ok(json) => json,
            Err(e) => {
                eprintln!(
                    "⚠ Warning: Could not parse existing settings.json: {}",
                    e
                );
                eprintln!("  Creating backup at settings.json.bak and starting fresh");
                let backup_path = settings_dir.join("settings.json.bak");
                fs::copy(&settings_path, &backup_path).ok();
                serde_json::json!({})
            }
        }
    } else {
        serde_json::json!({})
    };

    // Add hooks configuration using correct Claude Code format
    let hook_cmd = hook_path.to_string_lossy().to_string();

    // Claude Code hooks format requires:
    // "hooks": { "EventName": [{ "hooks": [{ "type": "command", "command": "..." }] }] }
    let hook_entry = serde_json::json!({
        "hooks": [{
            "type": "command",
            "command": hook_cmd
        }]
    });

    if let Some(obj) = settings.as_object_mut() {
        let hooks = obj.entry("hooks").or_insert(serde_json::json!({}));
        if let Some(hooks_obj) = hooks.as_object_mut() {
            // Add our hook to each event type
            for event in &["UserPromptSubmit", "PreToolUse", "PostToolUse", "Notification", "Stop"] {
                let event_hooks = hooks_obj.entry(*event).or_insert(serde_json::json!([]));
                if let Some(arr) = event_hooks.as_array_mut() {
                    // Check if our hook is already there
                    let hook_exists = arr.iter().any(|v| {
                        v.get("hooks")
                            .and_then(|h| h.as_array())
                            .map(|hooks_arr| {
                                hooks_arr.iter().any(|hook| {
                                    hook.get("command")
                                        .and_then(|c| c.as_str())
                                        .map(|s| s.contains("claude-deck"))
                                        .unwrap_or(false)
                                })
                            })
                            .unwrap_or(false)
                    });
                    if !hook_exists {
                        arr.push(hook_entry.clone());
                    }
                }
            }
        }
    }

    // Write settings back
    let settings_content = serde_json::to_string_pretty(&settings)
        .context("Failed to serialize settings to JSON")?;
    fs::write(&settings_path, settings_content)
        .with_context(|| format!("Failed to write settings to {:?}", settings_path))?;

    println!("✓ Updated Claude Code settings at {:?}", settings_path);
    println!();
    println!("Claude Code hooks installed successfully!");
    println!("The LCD strip will now show real-time status from Claude Code.");
    println!();
    println!("Note: You may need to restart Claude Code for hooks to take effect.");

    Ok(())
}

async fn check_status() -> Result<()> {
    use claude_deck::device::DeviceManager;

    info!("Checking device status...");

    match DeviceManager::find_device().await {
        Ok(info) => {
            println!("✓ Device found: {}", info.name);
            println!("  Firmware: {}", info.firmware_version);
            println!("  Serial: {}", info.serial_number);
            Ok(())
        }
        Err(e) => {
            println!("✗ No device found: {}", e);
            std::process::exit(1);
        }
    }
}

async fn set_brightness(brightness: u8) -> Result<()> {
    use claude_deck::device::DeviceManager;

    // Note: brightness is already validated by clap to be 0-100
    info!("Setting brightness to {}%", brightness);

    let manager = DeviceManager::connect().await?;
    manager.set_brightness(brightness).await?;
    println!("✓ Brightness set to {}%", brightness);
    Ok(())
}

fn uninstall_autostart() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        use std::fs;
        use std::path::PathBuf;

        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        let plist_path = PathBuf::from(&home).join("Library/LaunchAgents/com.claude-deck.plist");

        if plist_path.exists() {
            // Try to unload first (ignore errors if not loaded)
            let _ = std::process::Command::new("launchctl")
                .arg("unload")
                .arg(&plist_path)
                .output();

            fs::remove_file(&plist_path)
                .with_context(|| format!("Failed to remove {:?}", plist_path))?;
            println!("✓ Removed LaunchAgent at {:?}", plist_path);
        } else {
            println!("LaunchAgent not found (already uninstalled?)");
        }
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("Autostart uninstallation is only supported on macOS");
        Ok(())
    }
}

fn uninstall_hooks() -> Result<()> {
    use std::fs;
    use std::path::PathBuf;

    let home = std::env::var("HOME").context("HOME environment variable not set")?;

    // Remove hook script
    let hook_path = PathBuf::from(&home).join(".claude/hooks/claude-deck-hook.sh");
    if hook_path.exists() {
        fs::remove_file(&hook_path)
            .with_context(|| format!("Failed to remove hook script at {:?}", hook_path))?;
        println!("✓ Removed hook script at {:?}", hook_path);
    }

    // Remove hooks from settings
    let settings_path = PathBuf::from(&home).join(".claude/settings.json");
    if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)
            .with_context(|| format!("Failed to read settings from {:?}", settings_path))?;

        if let Ok(mut settings) = serde_json::from_str::<serde_json::Value>(&content) {
            let mut modified = false;

            if let Some(hooks) = settings.get_mut("hooks").and_then(|h| h.as_object_mut()) {
                for event in &["UserPromptSubmit", "PreToolUse", "PostToolUse", "Notification", "Stop"] {
                    if let Some(event_hooks) = hooks.get_mut(*event).and_then(|e| e.as_array_mut()) {
                        let original_len = event_hooks.len();
                        event_hooks.retain(|v| {
                            !v.get("hooks")
                                .and_then(|h| h.as_array())
                                .map(|hooks_arr| {
                                    hooks_arr.iter().any(|hook| {
                                        hook.get("command")
                                            .and_then(|c| c.as_str())
                                            .map(|s| s.contains("claude-deck"))
                                            .unwrap_or(false)
                                    })
                                })
                                .unwrap_or(false)
                        });
                        if event_hooks.len() != original_len {
                            modified = true;
                        }
                    }
                }
            }

            if modified {
                let settings_content = serde_json::to_string_pretty(&settings)
                    .context("Failed to serialize settings")?;
                fs::write(&settings_path, settings_content)
                    .with_context(|| format!("Failed to write settings to {:?}", settings_path))?;
                println!("✓ Removed claude-deck hooks from settings");
            }
        }
    }

    println!();
    println!("Claude Code hooks uninstalled successfully!");
    println!("Note: You may need to restart Claude Code for changes to take effect.");

    Ok(())
}
