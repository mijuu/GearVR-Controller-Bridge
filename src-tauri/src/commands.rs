//! Tauri commands
//! This module defines all the commands that can be invoked from the frontend.

use crate::core::bluetooth::BluetoothDevice;
use crate::state::AppState;
use tauri::{State, Window};

/// Scans for Bluetooth devices
///
/// # Arguments
/// * `duration_secs` - The duration of the scan in seconds
/// * `state` - The application state
///
/// # Returns
/// A list of discovered Bluetooth devices
#[tauri::command]
pub async fn scan_devices(
    duration_secs: Option<u64>,
    state: State<'_, AppState>,
) -> Result<Vec<BluetoothDevice>, String> {
    // Default scan duration is 5 seconds
    let duration = duration_secs.unwrap_or(5);
    
    // Initialize Bluetooth if not already initialized
    if state.bluetooth().is_err() {
        state.init_bluetooth().await.map_err(|e| e.to_string())?;
    }
    
    // Perform the scan
    // Clone the BluetoothManager to avoid holding the MutexGuard across an await point
    let bluetooth_manager = {
        let guard = state.bluetooth().map_err(|e| e.to_string())?;
        guard.as_ref().unwrap().clone()
    };
    
    bluetooth_manager.scan_devices(duration).await.map_err(|e| e.to_string())
}

/// Connects to a Bluetooth device
///
/// # Arguments
/// * `address` - The address of the device to connect to
/// * `window` - The Tauri window
/// * `state` - The application state
#[tauri::command]
pub async fn connect_to_device(
    address: String,
    window: Window,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Clone the BluetoothManager to avoid holding the MutexGuard across an await point
    let bluetooth_manager = {
        let guard = state.bluetooth().map_err(|e| e.to_string())?;
        guard.as_ref().unwrap().clone()
    };
    
    bluetooth_manager.connect_device(&address, window).await.map_err(|e| e.to_string())
}

/// Disconnects from the currently connected device
///
/// # Arguments
/// * `state` - The application state
#[tauri::command]
pub async fn disconnect(state: State<'_, AppState>) -> Result<(), String> {
    // Clone the BluetoothManager to avoid holding the MutexGuard across an await point
    let bluetooth_manager = {
        let guard = state.bluetooth().map_err(|e| e.to_string())?;
        guard.as_ref().unwrap().clone()
    };
    
    bluetooth_manager.disconnect().await.map_err(|e| e.to_string())
}