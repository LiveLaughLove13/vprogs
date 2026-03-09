//! Settings management for the wallet GUI
//!
//! Handles loading and saving wallet settings to a JSON configuration file.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Wallet settings structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub rpc_url: Option<String>,
    pub network: String, // "mainnet" or "testnet"
    pub default_privacy_mode: String, // "public", "confidential", "full_privacy"
    pub auto_scan: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            rpc_url: Some("ws://127.0.0.1:17110".to_string()),
            network: "mainnet".to_string(),
            default_privacy_mode: "public".to_string(),
            auto_scan: true,
        }
    }
}

/// Settings manager
pub struct SettingsManager {
    config_path: PathBuf,
}

impl SettingsManager {
    /// Create a new settings manager
    pub fn new(config_path: PathBuf) -> Self {
        Self { config_path }
    }

    /// Load settings from file, or return defaults if file doesn't exist
    pub fn load(&self) -> Result<Settings, String> {
        if !self.config_path.exists() {
            return Ok(Settings::default());
        }

        let content = fs::read_to_string(&self.config_path)
            .map_err(|e| format!("Failed to read settings file: {}", e))?;

        let settings: Settings = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse settings: {}", e))?;

        Ok(settings)
    }

    /// Save settings to file
    pub fn save(&self, settings: &Settings) -> Result<(), String> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create settings directory: {}", e))?;
        }

        let content = serde_json::to_string_pretty(settings)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;

        fs::write(&self.config_path, content)
            .map_err(|e| format!("Failed to write settings file: {}", e))?;

        Ok(())
    }

    /// Get the config file path
    #[allow(dead_code)] // Reserved for future use
    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }
}

