//! Defines shared data structures for the Bluetooth module.

use bluest::{Device, Characteristic};

/// Represents a discovered Bluetooth device
#[derive(Debug, Clone, serde::Serialize)]
pub struct BluetoothDevice {
    /// The name of the device, if available
    pub name: Option<String>,
    /// The address of the device (MAC address on most platforms, may be 00:00:00:00:00:00 on macOS)
    pub address: String,
    /// Platform-specific unique identifier for the device (especially important on macOS)
    pub id: String,
    /// The signal strength (RSSI) of the device
    pub rssi: Option<i16>,
    /// The battery level of the device, if available
    pub battery_level: Option<u8>,
    /// Whether the device is paired
    pub is_paired: bool,
    /// Whether the device is connected
    pub is_connected: bool,
}

impl BluetoothDevice {
    /// Creates a new BluetoothDevice instance
    pub fn new(name: Option<String>, address: String, id: String, rssi: Option<i16>, battery_level: Option<u8>, is_paired: bool, is_connected: bool) -> Self {
        Self {
            name,
            address,
            id,
            rssi,
            battery_level,
            is_paired,
            is_connected,
        }
    }

    /// Returns true if this device is a GearVR Controller
    pub fn is_gear_vr_controller(&self, controller_name: &str) -> bool {
        self.name
            .as_ref()
            .map(|name| name.contains(controller_name))
            .unwrap_or(false)
    }
}

/// Represents the state of a successfully connected device.
/// This struct holds the active handles needed for interaction.
#[derive(Clone)]
pub struct ConnectedDeviceState {
    /// The device handle, used for things like checking connection status or disconnecting.
    pub device: Device,
    /// The characteristic handle for receiving notifications from the device.
    pub notify_characteristic: Characteristic,
    /// The characteristic handle for writing commands to the device.
    pub write_characteristic: Characteristic,
}