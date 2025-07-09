//! Tauri commands
//! This module defines all the commands that can be invoked from the frontend.

use crate::state::AppState;
use crate::config::controller_config::ControllerConfig;
use crate::config::keymap_config::KeymapConfig;
use crate::config::mouse_config::MouseConfig;
use anyhow::{Result};
use tauri::{AppHandle, State, Window};
use log::{error};

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

    let mouse_sender_guard = app_state.mouse_sender.lock().await;
    let mouse_sender_clone = mouse_sender_guard.clone();
    
    bluetooth_manager_guard.connect_device(window, &device_id, mouse_sender_clone).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_battery_level(
    window: Window,
    app_state: State<'_, AppState>,
) -> Result<u8, String> {
    let bluetooth_manager_arc = app_state.bluetooth_manager.clone();
    let mut bluetooth_manager_guard = bluetooth_manager_arc.lock().await;
    
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
pub async fn disconnect(app_state: State<'_, AppState>) -> Result<(), String> {
    let bluetooth_manager_arc = app_state.bluetooth_manager.clone();
    let mut bluetooth_manager_guard = bluetooth_manager_arc.lock().await;
    
    bluetooth_manager_guard.disconnect().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reconnect_to_device(
    window: Window,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    let bluetooth_manager_arc = app_state.bluetooth_manager.clone();
    let mut bluetooth_manager_guard = bluetooth_manager_arc.lock().await;
    
    bluetooth_manager_guard.reconnect_device(window).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn turn_off_controller(app_state: State<'_, AppState>) -> Result<(), String> {
    let bluetooth_manager_arc = app_state.bluetooth_manager.clone();
    let bluetooth_manager_guard = bluetooth_manager_arc.lock().await;
    
    bluetooth_manager_guard.turn_off_controller().await.map_err(|e| e.to_string())
}

/// Starts the magnetometer calibration wizard.
#[tauri::command]
pub async fn start_mag_calibration_wizard(
    window: Window,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    let bluetooth_manager_arc = app_state.bluetooth_manager.clone();
    let bluetooth_manager_guard = bluetooth_manager_arc.lock().await;

    bluetooth_manager_guard.start_mag_calibration_wizard(window).await.map_err(|e| e.to_string())
}

/// Starts the gyroscope calibration.
#[tauri::command]
pub async fn start_gyro_calibration(
    window: Window,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    let bluetooth_manager_arc = app_state.bluetooth_manager.clone();
    let bluetooth_manager_guard = bluetooth_manager_arc.lock().await;

    bluetooth_manager_guard.start_gyro_calibration(window).await.map_err(|e| e.to_string())
}

/// Gets the current controller configuration.
#[tauri::command]
pub async fn get_controller_config(
    app_state: State<'_, AppState>,
) -> Result<ControllerConfig, String> {
    let bluetooth_manager_arc = app_state.bluetooth_manager.clone();
    let bluetooth_manager_guard = bluetooth_manager_arc.lock().await;

    Ok(bluetooth_manager_guard.notification_handler.get_controller_parser().lock().await.config.clone())
}

/// Sets the controller configuration.
#[tauri::command]
pub async fn set_controller_config(
    app_handle: AppHandle,
    app_state: State<'_, AppState>,
    config: ControllerConfig,
) -> Result<(), String> {
    let bluetooth_manager_arc = app_state.bluetooth_manager.clone();
    let bluetooth_manager_guard = bluetooth_manager_arc.lock().await;
    let controller_parser_arc  = bluetooth_manager_guard
        .notification_handler
        .get_controller_parser();
    let mut controller_parser_guard = controller_parser_arc.lock().await;

    // Update the config and re-initialize components within the parser
    controller_parser_guard.update_config(config);

    // Save the new config to disk
    if let Err(e) = controller_parser_guard.config.save_config(&app_handle).await {
        error!("Failed to save controller config: {}", e)
    }

    Ok(())
}

/// Resets the controller configuration to its default values.
#[tauri::command]
pub async fn reset_controller_config(
    app_handle: AppHandle,
    app_state: State<'_, AppState>,
) -> Result<ControllerConfig, String> {
    let bluetooth_manager_arc = app_state.bluetooth_manager.clone();
    let bluetooth_manager_guard = bluetooth_manager_arc.lock().await;
    let controller_parser_arc = bluetooth_manager_guard
        .notification_handler
        .get_controller_parser();
    let mut controller_parser_guard = controller_parser_arc.lock().await;

    // Reset to default config
    let new_config = ControllerConfig::default();
    controller_parser_guard.update_config(new_config.clone());

    // Save the new config to disk
    if let Err(e) = controller_parser_guard.config.save_config(&app_handle).await {
        error!("Failed to save controller config after reset: {}", e);
    }

    Ok(new_config)
}

// --- MouseConfig Commands ---

#[tauri::command]
pub async fn get_mouse_config(
    app_state: State<'_, AppState>,
) -> Result<MouseConfig, String> {
    let mouse_sender_arc = app_state.mouse_sender.clone();
    let mouse_sender_guard = mouse_sender_arc.lock().await;
    Ok(mouse_sender_guard.mouse_config.clone())
}

#[tauri::command]
pub async fn set_mouse_config(
    app_handle: AppHandle,
    app_state: State<'_, AppState>,
    config: MouseConfig,
) -> Result<(), String> {
    let mouse_sender_arc = app_state.mouse_sender.clone();
    let mut mouse_sender_guard = mouse_sender_arc.lock().await;

    mouse_sender_guard.mouse_config = config.clone();

    mouse_sender_guard.update_mouse_config(
        config.clone()
    ).await;

    if let Err(e) = mouse_sender_guard.mouse_config.save_config(&app_handle).await {
        error!("Failed to save mouse config: {}", e);
    }

    Ok(())
}

#[tauri::command]
pub async fn reset_mouse_config(
    app_handle: AppHandle,
    app_state: State<'_, AppState>,
) -> Result<MouseConfig, String> {
    let mouse_sender_arc = app_state.mouse_sender.clone();
    let mut mouse_sender_guard = mouse_sender_arc.lock().await;

    let new_config = MouseConfig::default();
    mouse_sender_guard.mouse_config = new_config.clone();

    mouse_sender_guard.update_mouse_config(
        new_config.clone()
    ).await;


    if let Err(e) = mouse_sender_guard.mouse_config.save_config(&app_handle).await {
        error!("Failed to save mouse config after reset: {}", e);
    }

    Ok(new_config)
}

// --- KeymapConfig Commands ---

#[tauri::command]
pub async fn get_keymap_config(
    app_state: State<'_, AppState>,
) -> Result<KeymapConfig, String> {
    let mouse_sender_arc = app_state.mouse_sender.clone();
    let mouse_sender_guard = mouse_sender_arc.lock().await;
    Ok(mouse_sender_guard.keymap_config.clone())
}

#[tauri::command]
pub async fn set_keymap_config(
    app_handle: AppHandle,
    app_state: State<'_, AppState>,
    config: KeymapConfig,
) -> Result<(), String> {
    let mouse_sender_arc = app_state.mouse_sender.clone();
    let mut mouse_sender_guard = mouse_sender_arc.lock().await;

    mouse_sender_guard.keymap_config = config.clone();

    mouse_sender_guard.update_keymap_config(
        config.clone()
    ).await;

    if let Err(e) = mouse_sender_guard.keymap_config.save_config(&app_handle).await {
        error!("Failed to save keymap config: {}", e);
    }

    Ok(())
}

#[tauri::command]
pub async fn reset_keymap_config(
    app_handle: AppHandle,
    app_state: State<'_, AppState>,
) -> Result<KeymapConfig, String> {
    let mouse_sender_arc = app_state.mouse_sender.clone();
    let mut mouse_sender_guard = mouse_sender_arc.lock().await;

    let new_config = KeymapConfig::default();
    mouse_sender_guard.keymap_config = new_config.clone();

    mouse_sender_guard.update_keymap_config(
        new_config.clone()
    ).await;

    if let Err(e) = mouse_sender_guard.keymap_config.save_config(&app_handle).await {
        error!("Failed to save keymap config after reset: {}", e);
    }

    Ok(new_config)
}


#[macro_export]
macro_rules! export_commands {
    () => {
        tauri::generate_handler![
            $crate::commands::start_scan,
            $crate::commands::stop_scan,
            $crate::commands::connect_to_device,
            $crate::commands::reconnect_to_device,
            $crate::commands::get_battery_level,
            $crate::commands::disconnect,
            $crate::commands::turn_off_controller,
            $crate::commands::start_mag_calibration_wizard,
            $crate::commands::start_gyro_calibration,
            $crate::commands::get_controller_config,
            $crate::commands::set_controller_config,
            $crate::commands::reset_controller_config,
            $crate::commands::get_mouse_config,
            $crate::commands::set_mouse_config,
            $crate::commands::reset_mouse_config,
            $crate::commands::get_keymap_config,
            $crate::commands::set_keymap_config,
            $crate::commands::reset_keymap_config
        ]
    };
}