//! GearVR Controller Bridge library
//! This is the main library for the GearVR Controller Bridge application.

// Module declarations
pub mod core;
pub mod commands;
pub mod state;
pub mod logging;

// Import our modules
use commands::{connect_to_device, disconnect, scan_devices_realtime, check_device_status, read_sensor_data};
use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // Register our commands
        .invoke_handler(tauri::generate_handler![
            scan_devices_realtime,
            connect_to_device,
            disconnect,
            check_device_status,
            read_sensor_data,
        ])
        // Setup our application state
        .setup(|app| {
            // Create and manage our application state
            app.manage(AppState::new());
            
            // 初始化自定义日志处理器
            if let Err(_) = logging::TauriLogger::init(app.handle().clone(), log::Level::Debug) {
                // 只有在TauriLogger初始化失败时才使用env_logger作为后备
                env_logger::builder()
                    .filter_level(log::LevelFilter::Debug)
                    .init();
            }
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}