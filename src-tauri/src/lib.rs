//! GearVR Controller Bridge library
//! This is the main library for the GearVR Controller Bridge application.

// Module declarations
pub mod core;
pub mod commands;
pub mod state;

// Import our modules
use commands::{connect_to_device, disconnect, scan_devices};
use state::AppState;
use tauri::Manager;

// Initialize logging
fn setup_logging() {
    env_logger::init();
    log::info!("Logging initialized");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    setup_logging();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // Register our commands
        .invoke_handler(tauri::generate_handler![
            scan_devices,
            connect_to_device,
            disconnect
        ])
        // Setup our application state
        .setup(|app| {
            // Create and manage our application state
            app.manage(AppState::new());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}