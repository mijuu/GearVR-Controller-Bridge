//! GearVR Controller Bridge library
//! This is the main library for the GearVR Controller Bridge application.

// Module declarations
pub mod core;
pub mod mapping;
pub mod commands;
pub mod state;
pub mod logging;

// Import our modules
use commands::{connect_to_device, disconnect, start_scan, stop_scan, get_battery_level, turn_off_controller};
use state::AppState;
use log::{info};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // Register our commands
        .invoke_handler(tauri::generate_handler![
            start_scan,
            stop_scan,
            connect_to_device,
            get_battery_level,
            disconnect,
            turn_off_controller,
        ])
        // Setup our application state
        .setup(move |app| {
            // Create and manage our application state
            let app_state_instance = rt.block_on(async {
                info!("Starting AppState initialization in Tauri setup.");
                AppState::new().await.expect("Failed to initialize AppState with BluetoothManager")
            });

            app.manage(app_state_instance);
            
            // 初始化自定义日志处理器
            if let Err(_) = logging::TauriLogger::init(app.handle().clone(), log::Level::Info) {
                // 只有在TauriLogger初始化失败时才使用env_logger作为后备
                env_logger::builder()
                    .filter_level(log::LevelFilter::Info)
                    .init();
            }
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}