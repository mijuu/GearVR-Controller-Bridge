//! Tauri commands
//! This module defines all the commands that can be invoked from the frontend.

use crate::state::AppState;
use anyhow::{Result};
use tauri::{State, Window};

/// Scans for Bluetooth devices with real-time updates through events
///
/// # Arguments
/// * `window` - The Tauri window
/// * `state` - The application state
///
/// # Returns
/// Nothing, but emits events during scanning:
/// - "scan-start" when scanning is started
/// - "device-found" with device details when a device is discovered
/// - "scan-complete" when scanning is finished
#[tauri::command]
pub async fn start_scan(
    window: Window,
    app_state: State<'_, AppState>,
) -> Result<(), String> {    
    let bluetooth_manager_arc = app_state.bluetooth_manager.clone();
    let mut bluetooth_manager_guard = bluetooth_manager_arc.lock().await;
    
    bluetooth_manager_guard.start_scan(window).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_scan(
    window: Window,
    app_state: State<'_, AppState>,
) -> Result<(), String> {    
    let bluetooth_manager_arc = app_state.bluetooth_manager.clone();
    let mut bluetooth_manager_guard = bluetooth_manager_arc.lock().await;
    
    bluetooth_manager_guard.stop_scan(window).await.map_err(|e| e.to_string())
}


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
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    let bluetooth_manager_arc = app_state.bluetooth_manager.clone();
    let mut bluetooth_manager_guard = bluetooth_manager_arc.lock().await;

    let mouse_sender = app_state.mouse_sender.clone(); 
    
    bluetooth_manager_guard.connect_device(window, &device_id, mouse_sender).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_battery_level(
    window: Window,
    app_state: State<'_, AppState>,
) -> Result<u8, String> {
    let bluetooth_manager_arc = app_state.bluetooth_manager.clone();
    let bluetooth_manager_guard = bluetooth_manager_arc.lock().await;
    
    bluetooth_manager_guard.get_battery_level(window)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No battery level available".to_string())
}

/// Disconnects from the currently connected device
///
/// # Arguments
/// * `state` - The application state
#[tauri::command]
pub async fn disconnect(window: Window, device_id: String, app_state: State<'_, AppState>) -> Result<(), String> {
    let bluetooth_manager_arc = app_state.bluetooth_manager.clone();
    let mut bluetooth_manager_guard = bluetooth_manager_arc.lock().await;
    
    bluetooth_manager_guard.disconnect(window, &device_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn turn_off_controller(app_state: State<'_, AppState>) -> Result<(), String> {
    let bluetooth_manager_arc = app_state.bluetooth_manager.clone();
    let bluetooth_manager_guard = bluetooth_manager_arc.lock().await;
    
    bluetooth_manager_guard.turn_off_controller().await.map_err(|e| e.to_string())
}