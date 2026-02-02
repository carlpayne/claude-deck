//! System utilities for macOS integration

use tokio::process::Command;
use tracing::{trace, warn};

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
        trace!("Focused app: {}", app_name);
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
