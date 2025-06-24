//! Application state management
//! This module defines and manages the global application state.

use std::sync::{Arc};
use tokio::sync::Mutex;
use anyhow::{Result};
use log::{info};
use crate::core::BluetoothManager;

/// Global application state
pub struct AppState {
    /// The Bluetooth manager instance
    pub bluetooth_manager: Arc<Mutex<BluetoothManager>>,
}

impl AppState {
    /// Creates a new AppState instance
    pub async fn new() -> Result<Self> {
        info!("Initializing BluetoothManager...");
        let manager = BluetoothManager::new().await?;
        Ok(Self {
            bluetooth_manager: Arc::new(Mutex::new(manager)),
        })
    }

    /// Gets a reference to the Bluetooth manager
    pub fn get_bluetooth_manager_arc(&self) -> Arc<Mutex<BluetoothManager>> {
        self.bluetooth_manager.clone()
    }
}