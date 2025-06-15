//! Application state management
//! This module defines and manages the global application state.

use std::sync::Mutex;
use crate::core::BluetoothManager;

/// Global application state
pub struct AppState {
    /// The Bluetooth manager instance
    bluetooth: Mutex<Option<BluetoothManager>>,
}

impl AppState {
    /// Creates a new AppState instance
    pub fn new() -> Self {
        Self {
            bluetooth: Mutex::new(None),
        }
    }

    /// Initializes the Bluetooth manager
    pub async fn init_bluetooth(&self) -> Result<(), Box<dyn std::error::Error>> {
        let bluetooth = BluetoothManager::new().await?;
        *self.bluetooth.lock().unwrap() = Some(bluetooth);
        Ok(())
    }

    /// Gets a reference to the Bluetooth manager
    pub fn bluetooth(&self) -> Result<std::sync::MutexGuard<Option<BluetoothManager>>, Box<dyn std::error::Error>> {
        let guard = self.bluetooth.lock().unwrap();
        if guard.is_none() {
            return Err("Bluetooth manager not initialized".into());
        }
        Ok(guard)
    }
}

// Implement Default for AppState
impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}