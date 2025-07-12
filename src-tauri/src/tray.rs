//! Tray module for handling tray menu internationalization.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{TrayIcon, TrayIconBuilder},
    path::BaseDirectory,
    AppHandle, Manager, ActivationPolicy,
};
use log::error;
use crate::commands;

// Define a type for our translations
pub type Translations = HashMap<String, String>;

/// Creates the tray icon and its menu.
pub fn create_tray(app_handle: &AppHandle) -> Result<TrayIcon, Box<dyn std::error::Error>> {
    let lang = tauri::async_runtime::block_on(commands::get_current_language(app_handle.clone()))
        .unwrap_or_else(|_| "en".to_string());

    let icon_path = PathBuf::from("icons/tray.png");
    let custom_icon = Image::from_path(icon_path)?;

    let tray = TrayIconBuilder::new()
        .icon(custom_icon)
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

    Ok(tray)
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
