// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use gearvr_controller_bridge_lib::{logging, state::AppState, tray};
use tauri::{
    Manager, WindowEvent, ActivationPolicy
};
use log::{info};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        // Register our commands
        .invoke_handler(gearvr_controller_bridge_lib::export_commands!())
        // Setup our application state
        .setup(move |app| {
            let tray = tray::create_tray(app.handle()).expect("Failed to create tray");
            app.manage(tray);

            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

            // Create and manage our application state
            let app_state_instance =
                rt.block_on(async {
                    info!("Starting AppState initialization in Tauri setup.");
                    AppState::new(app.handle()).await
                })
                .map_err(|e| {
                    format!("Failed to initialize AppState with BluetoothManager: {}", e)
                })?;

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
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
                #[cfg(target_os = "macos")]
                window.app_handle().set_activation_policy(ActivationPolicy::Accessory).unwrap();
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
