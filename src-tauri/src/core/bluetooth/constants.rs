//! Constants used throughout the application
//! This module contains all the constant values used in the application,
//! such as UUIDs, timeouts, and other configuration values.

use uuid::Uuid;

/// The name of the GearVR Controller
pub const CONTROLLER_NAME: &str = "Gear VR Controller";

/// Standard Bluetooth Service UUIDs
pub const UUID_GENERIC_ACCESS_SERVICE: Uuid = Uuid::from_u128(0x00001800_0000_1000_8000_00805f9b34fb);
pub const UUID_DEVICE_INFORMATION_SERVICE: Uuid = Uuid::from_u128(0x0000180a_0000_1000_8000_00805f9b34fb);
pub const UUID_BATTERY_SERVICE: Uuid = Uuid::from_u128(0x0000180f_0000_1000_8000_00805f9b34fb);

/// Standard Bluetooth Characteristic UUIDs
pub const UUID_DEVICE_NAME: Uuid = Uuid::from_u128(0x00002a00_0000_1000_8000_00805f9b34fb);
pub const UUID_MANUFACTURER_NAME: Uuid = Uuid::from_u128(0x00002a29_0000_1000_8000_00805f9b34fb);
pub const UUID_MODEL_NUMBER: Uuid = Uuid::from_u128(0x00002a24_0000_1000_8000_00805f9b34fb);
pub const UUID_BATTERY_LEVEL: Uuid = Uuid::from_u128(0x00002a19_0000_1000_8000_00805f9b34fb);

/// The UUID of the GearVR Controller service (Oculus Threemote)
pub const UUID_CONTROLLER_SERVICE: Uuid = Uuid::from_u128(0x4f63756c_7573_2054_6872_65656d6f7465);

/// The UUID of the GearVR Controller notification characteristic
pub const UUID_CONTROLLER_NOTIFY_CHAR: Uuid = Uuid::from_u128(0xc8c51726_81bc_483b_a052_f7a14ea3d281);

/// The UUID of the GearVR Controller write characteristic
pub const UUID_CONTROLLER_WRITE_CHAR: Uuid = Uuid::from_u128(0xc8c51726_81bc_483b_a052_f7a14ea3d282);

/// Maximum number of connection retries
pub const MAX_CONNECT_RETRIES: usize = 5;

/// Delay between connection retries in milliseconds
pub const CONNECT_RETRY_DELAY_MS: u64 = 1000;

/// Timeout for Bluetooth operations in seconds
pub const BLUETOOTH_OPERATION_TIMEOUT_SECS: u64 = 10;

/// Scan duration in seconds
pub const DEFAULT_SCAN_DURATION_SECS: u64 = 5;

/// Controller data packet size in bytes
pub const CONTROLLER_DATA_PACKET_SIZE: usize = 20;

/// Controller command packet size in bytes
pub const CONTROLLER_COMMAND_PACKET_SIZE: usize = 20;

/// Controller keep-alive interval in seconds
pub const CONTROLLER_KEEPALIVE_INTERVAL_SECS: u64 = 5;