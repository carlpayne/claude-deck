use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tokio::fs;
use tracing::{debug, warn};

/// Status file location
pub fn status_file_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".claude-deck/state.json")
}

/// Status information from Claude Code hooks
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClaudeStatus {
    /// Current action/task being performed
    #[serde(default)]
    pub task: String,

    /// Detail about the current tool (file path, command, etc.)
    #[serde(default)]
    pub tool_detail: Option<String>,

    /// Whether Claude is waiting for user input/permission
    #[serde(default)]
    pub waiting_for_input: bool,

    /// Type of input being waited for (e.g., "permission", "question")
    #[serde(default)]
    pub input_type: Option<String>,

    /// Current model being used
    #[serde(default)]
    pub model: Option<String>,

    /// Whether Claude is currently processing
    #[serde(default)]
    pub processing: bool,

    /// Last error message (if any)
    #[serde(default)]
    pub error: Option<String>,

    /// Timestamp of last update (Unix epoch seconds)
    #[serde(default)]
    pub timestamp: u64,
}

impl ClaudeStatus {
    /// Check if status is stale (older than threshold)
    pub fn is_stale(&self, max_age: Duration) -> bool {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        now.saturating_sub(self.timestamp) > max_age.as_secs()
    }
}

/// Read status from the status file
pub async fn read_status() -> Result<Option<ClaudeStatus>> {
    let path = status_file_path();

    if !path.exists() {
        return Ok(None);
    }

    match fs::read_to_string(&path).await {
        Ok(content) => {
            match serde_json::from_str::<ClaudeStatus>(&content) {
                Ok(status) => {
                    // Check if status is too old (more than 30 seconds)
                    if status.is_stale(Duration::from_secs(30)) {
                        debug!("Status file is stale, ignoring");
                        return Ok(None);
                    }
                    Ok(Some(status))
                }
                Err(e) => {
                    warn!("Failed to parse status file: {}", e);
                    Ok(None)
                }
            }
        }
        Err(e) => {
            warn!("Failed to read status file: {}", e);
            Ok(None)
        }
    }
}

/// Write status to the status file (used by hook scripts)
#[allow(dead_code)]
pub async fn write_status(status: &ClaudeStatus) -> Result<()> {
    let path = status_file_path();
    let content = serde_json::to_string_pretty(status)?;
    fs::write(&path, content).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_serialization() {
        let status = ClaudeStatus {
            task: "Writing code".to_string(),
            waiting_for_input: true,
            input_type: Some("permission".to_string()),
            model: Some("opus".to_string()),
            cost: 0.05,
            tokens: 1500,
            processing: false,
            error: None,
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&status).unwrap();
        let parsed: ClaudeStatus = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.task, "Writing code");
        assert_eq!(parsed.model, Some("opus".to_string()));
    }
}
