//! Bluetooth functionality for the GearVR Controller Bridge
//! This module handles all bluetooth operations including scanning,
//! connecting, and receiving data from the GearVR controller.

mod commands;
mod connection;
mod constants;
mod device;
mod manager;
mod notification;

// Re-export types that should be publicly accessible
pub use commands::{CommandExecutor, CommandSender, ControllerCommand};
pub use connection::ConnectionManager;
pub use constants::*;  // Re-export all constants
pub use device::BluetoothDevice;
pub use manager::BluetoothManager;
pub use notification::NotificationHandler;