//! Tauri commands
//! This module defines all the commands that can be invoked from the frontend.

use crate::state::AppState;
use tauri::{Emitter, State, Window};

/// Connects to a Bluetooth device
///
/// # Arguments
/// * `device_id` - The unique identifier of the device to connect to (platform-specific ID)
/// * `window` - The Tauri window
/// * `state` - The application state
#[tauri::command]
pub async fn connect_to_device(
    device_id: String,
    window: Window,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Clone the BluetoothManager to avoid holding the MutexGuard across an await point
    let bluetooth_manager = {
        let guard = state.bluetooth().map_err(|e| e.to_string())?;
        guard.as_ref().unwrap().clone()
    };
    
    bluetooth_manager.connect_device(&device_id, window).await.map_err(|e| e.to_string())
}

/// Scans for Bluetooth devices with real-time updates through events
///
/// # Arguments
/// * `window` - The Tauri window
/// * `duration_secs` - The duration of the scan in seconds
/// * `state` - The application state
///
/// # Returns
/// Nothing, but emits events during scanning:
/// - "scan-start" when scanning is started
/// - "device-found" with device details when a device is discovered
/// - "scan-complete" when scanning is finished
#[tauri::command]
pub async fn scan_devices_realtime(
    window: Window,
    duration_secs: Option<u64>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Default scan duration is 5 seconds
    let duration = duration_secs.unwrap_or(5);
    
    // Initialize Bluetooth if not already initialized
    if state.bluetooth().is_err() {
        state.init_bluetooth().await.map_err(|e| e.to_string())?;
    }
    
    // Clone the BluetoothManager to avoid holding the MutexGuard across an await point
    let bluetooth_manager = {
        let guard = state.bluetooth().map_err(|e| e.to_string())?;
        guard.as_ref().unwrap().clone()
    };
    
    // Emit scan-start event
    if let Err(e) = window.emit("scan-start", ()) {
        eprintln!("Failed to emit scan-start event: {}", e);
    }
    
    // Perform the real-time scan
    bluetooth_manager.scan_devices_realtime(window, duration).await.map_err(|e| e.to_string())
}

/// Disconnects from the currently connected device
///
/// # Arguments
/// * `state` - The application state
#[tauri::command]
pub async fn disconnect(device_id: String, state: State<'_, AppState>) -> Result<(), String> {
    // Clone the BluetoothManager to avoid holding the MutexGuard across an await point
    let bluetooth_manager = {
        let guard = state.bluetooth().map_err(|e| e.to_string())?;
        guard.as_ref().unwrap().clone()
    };
    
    bluetooth_manager.disconnect(&device_id).await.map_err(|e| e.to_string())
}


#[tauri::command]
pub async fn check_device_status(device_id: String, state: State<'_, AppState>) -> Result<(), String> {
    // Clone the BluetoothManager to avoid holding the MutexGuard across an await point
    let bluetooth_manager = {
        let guard = state.bluetooth().map_err(|e| e.to_string())?;
        guard.as_ref().unwrap().clone()
    };
    
    bluetooth_manager.check_controller_status(&device_id).await.map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn read_sensor_data(window: Window, state: State<'_, AppState>) -> Result<(), String> {
    // Clone the BluetoothManager to avoid holding the MutexGuard across an await point
    let bluetooth_manager = {
        let guard = state.bluetooth().map_err(|e| e.to_string())?;
        guard.as_ref().unwrap().clone()
    };
    
    bluetooth_manager.read_controller_data(window).await.map_err(|e| e.to_string())
}
