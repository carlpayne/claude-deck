//! System utilities for macOS integration

use tokio::process::Command;
use tracing::warn;

/// Get the name of the currently focused application on macOS
#[cfg(target_os = "macos")]
pub async fn get_focused_app() -> Option<String> {
    let output = match Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get name of first process whose frontmost is true")
        .output()
        .await
    {
        Ok(output) => output,
        Err(e) => {
            warn!("osascript command failed: {}", e);
            return None;
        }
    };

    if output.status.success() {
        let app_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Some(app_name)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("osascript failed: {} - {}", output.status, stderr);
        None
    }
}

#[cfg(not(target_os = "macos"))]
pub async fn get_focused_app() -> Option<String> {
    None
}

/// Check if the macOS screen is locked via IOConsoleLocked (~28ms)
#[cfg(target_os = "macos")]
pub async fn is_screen_locked() -> bool {
    let output = Command::new("sh")
        .args(["-c", "ioreg -n Root -d1 | grep -q '\"IOConsoleLocked\" = Yes'"])
        .output()
        .await;

    matches!(output, Ok(o) if o.status.success())
}

#[cfg(not(target_os = "macos"))]
pub async fn is_screen_locked() -> bool {
    false
}
