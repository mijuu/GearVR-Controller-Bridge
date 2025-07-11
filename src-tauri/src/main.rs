// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use gearvr_controller_bridge_lib::{logging, state::AppState};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    image::Image,
    Manager, WindowEvent, ActivationPolicy,
};
use std::path::PathBuf;
use log::info;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // Register our commands
        .invoke_handler(gearvr_controller_bridge_lib::export_commands!())
        // Setup our application state
        .setup(move |app| {
            let show_i = MenuItem::with_id(app, "show", "显示", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            let icon_path = PathBuf::from("icons/tray.png");
            let custom_icon = match Image::from_path(icon_path) {
                Ok(icon) => icon,
                Err(e) => {
                    eprintln!("Error loading icon: {}", e);
                    app.default_window_icon().unwrap().clone()
                }
            };
            TrayIconBuilder::new()
                .icon(custom_icon)
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            #[cfg(target_os = "macos")]
                            app.set_activation_policy(ActivationPolicy::Regular).unwrap();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

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
