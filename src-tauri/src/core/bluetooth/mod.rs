//! Bluetooth functionality for the GearVR Controller Bridge
//! This module handles all bluetooth operations including scanning,
//! connecting, and receiving data from the GearVR controller.

mod commands;
mod connection;
mod constants;
mod manager;
mod notification;
mod scanner;
mod types;

// Re-export types that should be publicly accessible
pub use commands::{CommandExecutor, CommandSender, ControllerCommand};
pub use connection::ConnectionManager;
pub use constants::*; // Re-export all constants
pub use manager::BluetoothManager;
pub use notification::NotificationHandler;
pub use scanner::BluetoothScanner;
pub use types::{BluetoothDevice, ConnectedDeviceState};
