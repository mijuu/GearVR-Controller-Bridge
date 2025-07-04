//! GearVR Controller Bridge library
//! This is the main library for the GearVR Controller Bridge application.

// Module declarations
pub mod core;
pub mod mapping;
pub mod state;
pub mod config;
pub mod commands;
pub mod logging;
pub mod utils;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
}