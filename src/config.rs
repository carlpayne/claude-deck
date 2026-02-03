use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::profiles::store::ProfileConfig;

/// Application configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub device: DeviceConfig,
    pub yolo: YoloConfig,
    pub new_session: NewSessionConfig,
    pub appearance: AppearanceConfig,
    pub models: ModelsConfig,
    pub web: WebConfig,
    pub giphy: GiphyConfig,
    #[serde(default)]
    pub profiles: Vec<ProfileConfig>,
}

impl Config {
    /// Load configuration from file or create default
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let contents = std::fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read config file at {:?}", config_path))?;
            let config: Config = toml::from_str(&contents)
                .with_context(|| format!("Failed to parse config file at {:?}", config_path))?;
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
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory at {:?}", parent))?;
        }

        let contents = toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(&config_path, contents)
            .with_context(|| format!("Failed to write config file at {:?}", config_path))?;
        Ok(())
    }

    /// Get config file path
    pub fn config_path() -> Result<PathBuf> {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        Ok(PathBuf::from(home).join(".config/claude-deck/config.toml"))
    }

    /// Get state file path (for hooks communication)
    pub fn state_path() -> Result<PathBuf> {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WebConfig {
    /// Enable the web configuration UI
    pub enabled: bool,
    /// Port for the web UI server
    pub port: u16,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 9845,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GiphyConfig {
    /// Giphy API key (uses default beta key if not specified)
    pub api_key: String,
}

impl Default for GiphyConfig {
    fn default() -> Self {
        Self {
            // Giphy's public beta API key - free tier, generous limits
            // Users can override with their own key in config if needed
            api_key: "dc6zaTOxFJmzC".to_string(),
        }
    }
}
