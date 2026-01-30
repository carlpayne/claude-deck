use anyhow::Result;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use crate::config::Config;
use crate::state::{AppState, InputType};

/// State update from hooks
#[derive(Debug, Deserialize)]
pub struct StateUpdate {
    pub task_name: Option<String>,
    pub progress: Option<u8>,
    pub waiting_for_input: Option<bool>,
    pub input_type: Option<String>,
    pub model: Option<String>,
}

/// Listens for hook output files and updates state
pub struct HooksListener {
    state: Arc<RwLock<AppState>>,
    #[allow(dead_code)]
    watcher: RecommendedWatcher,
    state_path: PathBuf,
}

impl HooksListener {
    /// Create a new hooks listener
    pub fn new(state: Arc<RwLock<AppState>>) -> Result<Self> {
        let state_path = Config::state_path()?;

        // Ensure parent directory exists
        if let Some(parent) = state_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Create watcher
        let state_clone = state.clone();
        let path_clone = state_path.clone();

        let watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    if event.kind.is_modify() || event.kind.is_create() {
                        // Read and parse state file
                        if let Ok(contents) = std::fs::read_to_string(&path_clone) {
                            if let Ok(update) = serde_json::from_str::<StateUpdate>(&contents) {
                                let state = state_clone.clone();
                                tokio::spawn(async move {
                                    Self::apply_update(&state, update).await;
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Watch error: {:?}", e);
                }
            }
        })?;

        info!("Hooks listener initialized, watching {:?}", state_path);

        Ok(Self {
            state,
            watcher,
            state_path,
        })
    }

    /// Start watching for state file changes
    pub fn start(&mut self) -> Result<()> {
        if let Some(parent) = self.state_path.parent() {
            self.watcher.watch(parent, RecursiveMode::NonRecursive)?;
            info!("Started watching {:?}", parent);
        }
        Ok(())
    }

    /// Apply a state update
    async fn apply_update(state: &Arc<RwLock<AppState>>, update: StateUpdate) {
        let mut state = state.write().await;

        if let Some(task) = update.task_name {
            debug!("Hook update: task_name = {}", task);
            state.task_name = task;
        }

        if let Some(progress) = update.progress {
            debug!("Hook update: progress = {}", progress);
            state.progress = progress.min(100);
        }

        if let Some(waiting) = update.waiting_for_input {
            debug!("Hook update: waiting_for_input = {}", waiting);
            state.waiting_for_input = waiting;
        }

        if let Some(input_type) = update.input_type {
            debug!("Hook update: input_type = {}", input_type);
            state.input_type = match input_type.as_str() {
                "yes_no" | "YesNo" => Some(InputType::YesNo),
                "continue" | "Continue" => Some(InputType::Continue),
                "permission" | "Permission" => Some(InputType::Permission),
                _ => None,
            };
        }

        if let Some(model) = update.model {
            debug!("Hook update: model = {}", model);
            state.set_model(&model);
        }
    }

    /// Manually read and apply current state file
    pub async fn read_current(&self) -> Result<()> {
        if self.state_path.exists() {
            let contents = std::fs::read_to_string(&self.state_path)?;
            let update: StateUpdate = serde_json::from_str(&contents)?;
            Self::apply_update(&self.state, update).await;
        }
        Ok(())
    }
}

/// Generate hook scripts for Claude Code
#[allow(dead_code)]
pub fn generate_hook_scripts() -> Result<()> {
    let home = std::env::var("HOME")?;
    let hooks_dir = PathBuf::from(&home).join(".claude/hooks");
    std::fs::create_dir_all(&hooks_dir)?;

    // Post-tool use hook
    let post_tool = r#"#!/bin/bash
# claude-deck hook: Called after each tool use
STATE_FILE="$HOME/.claude-deck/state.json"
mkdir -p "$(dirname "$STATE_FILE")"

# Parse hook input (JSON from stdin)
INPUT=$(cat)

# Extract relevant fields
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')

# Update state file
if [ -n "$TOOL_NAME" ]; then
    jq -n --arg task "$TOOL_NAME" '{task_name: $task}' > "$STATE_FILE"
fi
"#;

    let post_tool_path = hooks_dir.join("claude-deck-postToolUse.sh");
    std::fs::write(&post_tool_path, post_tool)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&post_tool_path, std::fs::Permissions::from_mode(0o755))?;
    }

    // Session start hook
    let session_start = r#"#!/bin/bash
# claude-deck hook: Called on session start
STATE_FILE="$HOME/.claude-deck/state.json"
mkdir -p "$(dirname "$STATE_FILE")"

# Reset state on new session
jq -n '{
  task_name: "READY",
  progress: 0
}' > "$STATE_FILE"
"#;

    let session_start_path = hooks_dir.join("claude-deck-sessionStart.sh");
    std::fs::write(&session_start_path, session_start)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&session_start_path, std::fs::Permissions::from_mode(0o755))?;
    }

    info!("Generated hook scripts in {:?}", hooks_dir);
    Ok(())
}
