use anyhow::Result;
use clap::Parser;
use tokio::signal;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use claude_deck::{config::Config, App};

#[derive(Parser, Debug)]
#[command(name = "claude-deck")]
#[command(about = "Hardware controller for Claude Code using AJAZZ AKP05E")]
#[command(version)]
struct Cli {
    /// Check device connection status and exit
    #[arg(long)]
    status: bool,

    /// Set device brightness (0-100)
    #[arg(long, value_name = "PERCENT")]
    brightness: Option<u8>,

    /// Install autostart on login (macOS LaunchAgent)
    #[arg(long)]
    install_autostart: bool,

    /// Install Claude Code hooks for status integration
    #[arg(long)]
    install_hooks: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Set up SIGCHLD handler to auto-reap child processes (prevents zombies)
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

    if cli.install_hooks {
        return install_hooks();
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

    // Run the application with graceful shutdown
    let mut app = App::new(config).await?;

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

        let home = std::env::var("HOME")?;
        let launch_agents = PathBuf::from(&home).join("Library/LaunchAgents");
        fs::create_dir_all(&launch_agents)?;

        let plist_path = launch_agents.join("com.claude-deck.plist");
        let binary_path = std::env::current_exe()?;

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

        fs::write(&plist_path, plist_content)?;
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

    let home = std::env::var("HOME")?;

    // Claude Code hooks directory
    let hooks_dir = PathBuf::from(&home).join(".claude/hooks");
    fs::create_dir_all(&hooks_dir)?;

    // Hook script content (embedded)
    let hook_script = include_str!("../hooks/claude-deck-hook.sh");
    let hook_path = hooks_dir.join("claude-deck-hook.sh");

    fs::write(&hook_path, hook_script)?;

    // Make executable
    let mut perms = fs::metadata(&hook_path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&hook_path, perms)?;

    println!("✓ Installed hook script at {:?}", hook_path);

    // Update Claude Code settings
    let settings_dir = PathBuf::from(&home).join(".claude");
    fs::create_dir_all(&settings_dir)?;

    let settings_path = settings_dir.join("settings.json");

    // Read existing settings or create new
    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
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
            for event in &["PreToolUse", "PostToolUse", "Notification", "Stop"] {
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
    let settings_content = serde_json::to_string_pretty(&settings)?;
    fs::write(&settings_path, settings_content)?;

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

    let brightness = brightness.min(100);
    info!("Setting brightness to {}%", brightness);

    let manager = DeviceManager::connect().await?;
    manager.set_brightness(brightness).await?;
    println!("✓ Brightness set to {}%", brightness);
    Ok(())
}
