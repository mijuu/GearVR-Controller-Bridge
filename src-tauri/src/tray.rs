//! Tray module for handling tray menu internationalization.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{TrayIcon, TrayIconBuilder},
    path::BaseDirectory,
    AppHandle, Manager, State,
};
use log::error;
use crate::commands;

#[cfg(target_os = "macos")]
use tauri::ActivationPolicy;

// Define a type for our translations
pub type Translations = HashMap<String, String>;

/// Creates the tray icon and its menu.
pub fn create_tray(app_handle: &AppHandle) -> Result<TrayIcon, Box<dyn std::error::Error>> {
    let lang = tauri::async_runtime::block_on(commands::get_current_language(app_handle.clone()))
        .unwrap_or_else(|_| "en".to_string());

    let tray_icon = get_tray_icon(app_handle)?;

    let tray = TrayIconBuilder::new()
        .icon(tray_icon)
        .icon_as_template(true)
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
        .build(app_handle)?;

    if let Err(e) = update_tray_menu(app_handle, &tray, &lang) {
        error!("Failed to set initial tray menu: {}", e);
    }

    // Add a theme change event listener
    if let Some(window) = app_handle.get_webview_window("main") {
        let app_handle = app_handle.clone();
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::ThemeChanged(_) = event {
                let tray_state: State<TrayIcon> = app_handle.state();
                if let Ok(icon) = get_tray_icon(&app_handle) {
                    if let Err(e) = tray_state.set_icon(Some(icon)) {
                        error!("Failed to set tray icon: {}", e);
                    }
                    #[cfg(target_os = "macos")]
                    tray_state.set_icon_as_template(true)
                        .expect("Failed to set tray icon as template");
                }
            }
        });
    }

    Ok(tray)
}

/// Gets the appropriate tray icon based on the current system theme.
fn get_tray_icon(app_handle: &AppHandle) -> Result<Image<'static>, Box<dyn std::error::Error>> {
    let icon_path = if cfg!(target_os = "macos") {
        PathBuf::from("icons/tray-dark.png")
    } else {
        let window = app_handle.get_webview_window("main").ok_or("Main window not found")?;
        let theme = window.theme()?;
        match theme {
            tauri::Theme::Light => PathBuf::from("icons/tray-light.png"),
            tauri::Theme::Dark => PathBuf::from("icons/tray-dark.png"),
            _ => PathBuf::from("icons/tray-dark.png"),
        }
    };
    Image::from_path(icon_path).map_err(|e| e.into())
}


/// Loads and flattens translations from a JSON file.
pub fn load_translations(app_handle: &AppHandle, lang: &str) -> Option<Translations> {
    let path = app_handle
        .path()
        .resolve(format!("locales/{}/translation.json", lang), BaseDirectory::Resource)
        .ok()?;
    let content = fs::read_to_string(path).ok()?;
    let v: serde_json::Value = match serde_json::from_str(&content) {
        Ok(value) => value,
        Err(e) => {
            error!("Failed to parse translation file for lang '{}': {}", lang, e);
            return None;
        }
    };

    let mut translations = HashMap::new();
    let mut stack: Vec<(String, serde_json::Value)> = vec![("".to_string(), v)];

    while let Some((prefix, value)) = stack.pop() {
        if let Some(map) = value.as_object() {
            for (key, val) in map.iter() {
                let new_prefix = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                stack.push((new_prefix, val.clone()));
            }
        } else if let Some(s) = value.as_str() {
            translations.insert(prefix, s.to_string());
        }
    }

    Some(translations)
}

/// Update tray menu with new text
pub fn update_tray_menu(app_handle: &AppHandle, tray: &TrayIcon, lang: &str) -> Result<(), String> {
    let translations =
        load_translations(app_handle, lang).or_else(|| load_translations(app_handle, "en")).unwrap_or_default();

    let show_text = translations
        .get("trayMenu.show")
        .map_or("Show", |s| s.as_str());
    let quit_text = translations
        .get("trayMenu.quit")
        .map_or("Quit", |s| s.as_str());
    
    let show_i = MenuItem::with_id(app_handle, "show", show_text, true, None::<&str>).map_err(|e| e.to_string())?;
    let quit_i = MenuItem::with_id(app_handle, "quit", quit_text, true, None::<&str>).map_err(|e| e.to_string())?;
    let menu = Menu::with_items(app_handle, &[&show_i, &quit_i]).map_err(|e| e.to_string())?;

    tray.set_menu(Some(menu)).map_err(|e| e.to_string())
}
