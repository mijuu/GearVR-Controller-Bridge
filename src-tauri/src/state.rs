//! Application state management
//! This module defines and manages the global application state.

use std::sync::{Arc};
use tokio::sync::{Mutex};
use anyhow::{Result};
use log::{info};
use crate::core::BluetoothManager;
use crate::mapping::mouse::MouseMapperSender;
use crate::config::controller_config::ControllerConfig;
use crate::config::mouse_mapper_config::MouseMapperConfig;
use tauri::AppHandle;

/// Global application state
pub struct AppState {
    /// The Bluetooth manager instance
    pub bluetooth_manager: Arc<Mutex<BluetoothManager>>,
    pub mouse_sender: MouseMapperSender,
}

impl AppState {
    /// Creates a new AppState instance
    pub async fn new(app_handle: &AppHandle) -> Result<Self> {
        info!("Initializing BluetoothManager...");

        let initial_controller_config = ControllerConfig::load_config(app_handle).await.ok();
        let initial_mouse_mapper_config = MouseMapperConfig::load_config(app_handle).await.ok();

        let manager = BluetoothManager::new(initial_controller_config).await?;
        Ok(Self {
            bluetooth_manager: Arc::new(Mutex::new(manager)),
            mouse_sender: MouseMapperSender::new(initial_mouse_mapper_config.unwrap_or_default()),
        })
    }

    /// Gets a reference to the Bluetooth manager
    pub fn get_bluetooth_manager_arc(&self) -> Arc<Mutex<BluetoothManager>> {
        self.bluetooth_manager.clone()
    }
}