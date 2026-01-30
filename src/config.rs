use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub device: DeviceConfig,
    pub yolo: YoloConfig,
    pub new_session: NewSessionConfig,
    pub appearance: AppearanceConfig,
    pub models: ModelsConfig,
}

impl Config {
    /// Load configuration from file or create default
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let contents = std::fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&contents)?;
            Ok(config)
        } else {
            // Create default config
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, contents)?;
        Ok(())
    }

    /// Get config file path
    pub fn config_path() -> Result<PathBuf> {
        let home = std::env::var("HOME")?;
        Ok(PathBuf::from(home).join(".config/claude-deck/config.toml"))
    }

    /// Get state file path (for hooks communication)
    pub fn state_path() -> Result<PathBuf> {
        let home = std::env::var("HOME")?;
        Ok(PathBuf::from(home).join(".claude-deck/state.json"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceConfig {
    /// Device brightness (0-100)
    pub brightness: u8,
    /// Seconds before dimming display
    pub idle_timeout: u32,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            brightness: 80,
            idle_timeout: 300,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct YoloConfig {
    /// Require long press to toggle YOLO mode
    pub require_long_press: bool,
    /// Long press duration in milliseconds
    pub long_press_duration_ms: u64,
}

impl Default for YoloConfig {
    fn default() -> Self {
        Self {
            require_long_press: true,
            long_press_duration_ms: 2000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NewSessionConfig {
    /// Terminal app to launch
    pub terminal: String,
}

impl Default for NewSessionConfig {
    fn default() -> Self {
        Self {
            terminal: "iTerm".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceConfig {
    /// Color theme
    pub theme: String,
    /// Accent color (hex)
    pub accent_color: String,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            accent_color: "#00ff88".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelsConfig {
    /// Available models for the model knob
    pub available: Vec<String>,
    /// Default model
    pub default: String,
}

impl Default for ModelsConfig {
    fn default() -> Self {
        Self {
            available: vec![
                "opus".to_string(),
                "sonnet".to_string(),
                "haiku".to_string(),
            ],
            default: "opus".to_string(),
        }
    }
}
