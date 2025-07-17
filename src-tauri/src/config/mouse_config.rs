use crate::utils::ensure_directory_exists;
use anyhow::Result;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tokio::fs;

const CONFIG_FILE_NAME: &str = "mouse_config.json";

/// Mouse movement mode
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MouseMode {
    /// Use controller air mouse to control mouse movement
    AirMouse,
    /// Use touchpad to control mouse movement (like laptop touchpad)
    Touchpad,
}

/// Mouse settings configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseConfig {
    /// Mouse movement mode
    pub mode: MouseMode,
    /// Mouse sensitivity for touchpad mode
    pub touchpad_sensitivity: f32,
    /// Acceleration factor for touchpad mode. 0.0 means no acceleration.
    pub touchpad_acceleration: f32,
    /// The speed threshold to activate acceleration. Below this, movement is linear (precise).
    /// The unit is abstract, related to (distance_squared / time_delta).
    pub touchpad_acceleration_threshold: f32,
    /// The horizontal field of view (in degrees) that maps to the full screen width.
    pub air_mouse_fov: f32,
    /// Rotational speed threshold (e.g., in degrees per second) to activate air mouse mode.
    pub air_mouse_activation_threshold: f32,
}

impl Default for MouseConfig {
    fn default() -> Self {
        MouseConfig {
            mode: MouseMode::Touchpad,
            touchpad_sensitivity: 500.0,
            touchpad_acceleration: 1.2,
            touchpad_acceleration_threshold: 0.0002,
            air_mouse_fov: 40.0,
            air_mouse_activation_threshold: 5.0,
        }
    }
}

impl MouseConfig {
    /// Loads the config from a configuration file.
    pub async fn load_config(app_handle: &AppHandle) -> Result<Self> {
        let config_dir = app_handle.path().app_config_dir()?;
        let file_path = config_dir.join(CONFIG_FILE_NAME);
        let file_path_str = file_path.to_string_lossy().into_owned();

        if !file_path.exists() {
            warn!(
                "Mouse config file not found at {:?}, using default.",
                file_path_str
            );
            return Ok(Self::default());
        }

        let config_json = fs::read_to_string(file_path).await?;
        let config: Self = serde_json::from_str(&config_json)?;

        info!("Mouse config loaded from {:?}", file_path_str);
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
                error!("Failed to serialize mouse config to JSON: {}", e);
                return Err(e.into());
            }
        };

        fs::write(file_path.to_path_buf(), config_json).await?;
        info!("Mouse config saved to {:?}", file_path_str);
        Ok(())
    }
}
