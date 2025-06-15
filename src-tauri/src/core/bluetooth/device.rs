//! Bluetooth device representation and related functionality

/// Represents a discovered Bluetooth device
#[derive(Debug, Clone, serde::Serialize)]
pub struct BluetoothDevice {
    /// The name of the device, if available
    pub name: Option<String>,
    /// The address of the device
    pub address: String,
    /// The signal strength (RSSI) of the device
    pub rssi: Option<i16>,
}

impl BluetoothDevice {
    /// Creates a new BluetoothDevice instance
    pub fn new(name: Option<String>, address: String, rssi: Option<i16>) -> Self {
        Self {
            name,
            address,
            rssi,
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