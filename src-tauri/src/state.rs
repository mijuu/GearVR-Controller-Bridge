//! Application state management
//! This module defines and manages the global application state.

use std::sync::{Arc};
use tokio::sync::{Mutex};
use anyhow::{Result};
use tauri::AppHandle;
use log::{info};
use crate::core::BluetoothManager;
use crate::mapping::mouse::MouseMapperSender;
use crate::config::controller_config::ControllerConfig;
use crate::config::mouse_config::MouseConfig;
use crate::config::keymap_config::KeymapConfig;

/// Global application state
pub struct AppState {
    /// The Bluetooth manager instance
    pub bluetooth_manager: Arc<Mutex<BluetoothManager>>,
    pub mouse_sender: Arc<Mutex<MouseMapperSender>>,
}

impl AppState {
    /// Creates a new AppState instance
    pub async fn new(app_handle: &AppHandle) -> Result<Self> {
        info!("Initializing BluetoothManager...");

        let initial_controller_config = ControllerConfig::load_config(app_handle).await.ok();
        let initial_mouse_config = MouseConfig::load_config(app_handle).await.ok();
        let initial_keymap_config = KeymapConfig::load_config(app_handle).await.ok();

        let bluetooth_manager = BluetoothManager::new(initial_controller_config.unwrap_or_default()).await?;
        let mouse_sender = MouseMapperSender::new(
            app_handle, 
            initial_mouse_config.unwrap_or_default(),
            initial_keymap_config.unwrap_or_default()
        );
        Ok(Self {
            bluetooth_manager: Arc::new(Mutex::new(bluetooth_manager)),
            mouse_sender: Arc::new(Mutex::new(mouse_sender)),
        })
    }

    /// Gets a reference to the Bluetooth manager
    pub fn get_bluetooth_manager_arc(&self) -> Arc<Mutex<BluetoothManager>> {
        self.bluetooth_manager.clone()
    }
}
