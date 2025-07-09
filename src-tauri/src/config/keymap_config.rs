use crate::utils::ensure_directory_exists;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use anyhow::Result;
use tokio::fs;
use log::{error, info, warn};

const CONFIG_FILE_NAME: &str = "keymap_config.json";

/// Configuration for button mappings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeymapConfig {
    /// Trigger button mapping
    pub trigger: Option<String>,
    /// Home button mapping
    pub home: Option<String>,
    /// Back button mapping
    pub back: Option<String>,
    /// Volume up button mapping
    pub volume_up: Option<String>,
    /// Volume down button mapping
    pub volume_down: Option<String>,
    /// Touchpad click mapping
    pub touchpad: Option<String>,
}

impl Default for KeymapConfig {
    fn default() -> Self {
        KeymapConfig {
            trigger: Some("Left".to_string()),
            home: Some("".to_string()),
            back: Some("Backspace".to_string()),
            volume_up: Some("Volume up".to_string()),
            volume_down: Some("Volume down".to_string()),
            touchpad: Some("Right".to_string()),
        }
    }
}

impl KeymapConfig {
    /// Loads the config from a configuration file.
    pub async fn load_config(app_handle: &AppHandle) -> Result<Self> {
        let config_dir = app_handle.path().app_config_dir()?;
        let file_path = config_dir.join(CONFIG_FILE_NAME);
        let file_path_str = file_path.to_string_lossy().into_owned();

        if !file_path.exists() {
            warn!("Keymap config file not found at {:?}, using default.", file_path_str);
            return Ok(Self::default());
        }

        let config_json = fs::read_to_string(file_path).await?;
        let config: Self = serde_json::from_str(&config_json)?;

        info!("Keymap config loaded from {:?}", file_path_str);
        Ok(config)
    }

    /// Saves the current config to a configuration file.
    pub async fn save_config(&self, app_handle: &AppHandle) -> Result<()> {
        let config_dir = app_handle.path().app_config_dir()?;
        ensure_directory_exists(&config_dir).await?;

        let file_path = config_dir.join(CONFIG_FILE_NAME);
        let file_path_str = file_path.to_string_lossy().into_owned();

        let config_json = match serde_json::to_string_pretty(&self) {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to serialize keymap config to JSON: {}", e);
                return Err(e.into());
            }
        };

        fs::write(file_path.to_path_buf(), config_json).await?;
        info!("Keymap config saved to {:?}", file_path_str);
        Ok(())
    }
}
