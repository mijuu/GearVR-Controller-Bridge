//! GearVR Controller Bridge library
//! This is the main library for the GearVR Controller Bridge application.

// Module declarations
pub mod commands;
pub mod config;
pub mod core;
pub mod logging;
pub mod mapping;
pub mod state;
pub mod tray;
pub mod utils;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {}
