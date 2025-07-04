// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use gearvr_controller_bridge_lib::{logging, state::AppState};
use tauri::{Manager, RunEvent};
use log::{info};

struct TokioRuntime(pub tokio::runtime::Runtime);
fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // Register our commands
        .invoke_handler(gearvr_controller_bridge_lib::export_commands!())
        // Setup our application state
        .setup(move |app| {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;
    
            // Create and manage our application state
            let app_state_instance = rt.block_on(async {
                info!("Starting AppState initialization in Tauri setup.");
                AppState::new(app.handle()).await
            }).map_err(|e| format!("Failed to initialize AppState with BluetoothManager: {}", e))?;

            app.manage(app_state_instance);
            app.manage(TokioRuntime(rt));
            
            // 初始化自定义日志处理器
            if let Err(_) = logging::TauriLogger::init(app.handle().clone(), log::Level::Info) {
                // 只有在TauriLogger初始化失败时才使用env_logger作为后备
                env_logger::builder()
                    .filter_level(log::LevelFilter::Info)
                    .init();
            }
            
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|app_handle, event| {
        if let RunEvent::ExitRequested { api, .. } = event {
            api.prevent_exit();

            // let state = app_handle.state::<AppState>();
            let rt_state = app_handle.state::<TokioRuntime>();
            let app_handle_clone = app_handle.clone();
    
            // 通知蓝牙管理器，应用即将退出
            rt_state.0.block_on(async {
                // let mut manager = state.bluetooth_manager.lock().await;

                // info!("Cleaning up connections...");
                // if let Err(e) = manager.disconnect().await {
                //     eprintln!("清理连接失败: {}", e);
                // }
            });
            
            // 异步清理完成后，安全退出应用
            app_handle_clone.exit(0);
        }
    })
}
