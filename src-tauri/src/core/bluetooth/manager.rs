//! Bluetooth manager for the GearVR Controller Bridge
//! This module provides the main interface for bluetooth operations

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};

use regex::Regex;
use anyhow::{anyhow, Result};
use bluest::{Adapter, Device};
use futures_util::StreamExt;
use log::{info, debug, error};
use tokio::time::timeout;
use tauri::{Window, Emitter};

use crate::core::bluetooth::commands::CommandExecutor;
use crate::core::bluetooth::connection::{ConnectionManager, BluestCommandSender};
use crate::core::bluetooth::types::{BluetoothDevice, ConnectedDeviceState};
use crate::core::bluetooth::notification::NotificationHandler;
use crate::core::bluetooth::constants::{
    CONTROLLER_NAME,
    MAX_CONNECT_RETRIES,
    CONNECT_RETRY_DELAY_MS,
    UUID_CONTROLLER_SERVICE,
    UUID_CONTROLLER_NOTIFY_CHAR,
    UUID_CONTROLLER_WRITE_CHAR,
    UUID_BATTERY_SERVICE,
    UUID_BATTERY_LEVEL,
    MIN_RSSI_THRESHOLD,
};
use crate::core::controller::ControllerParser;

/// Manages Bluetooth operations
#[derive(Clone)]
pub struct BluetoothManager {
    /// The Bluetooth adapter
    adapter: Adapter,
    /// Map of device addresses to devices
    devices: Arc<Mutex<HashMap<String, Device>>>,
    /// Currently connected device
    connected_state: Arc<Mutex<Option<ConnectedDeviceState>>>,
    /// Connection manager
    connection_manager: ConnectionManager,
    /// Notification handler
    notification_handler: NotificationHandler,
}

impl BluetoothManager {
    /// Creates a new BluetoothManager
    pub async fn new() -> Result<Self> {
        let adapter = Adapter::default().await
            .ok_or_else(|| anyhow!("No Bluetooth adapter found"))?;
        adapter.wait_available().await?;
        info!("Bluetooth adapter is available.");

        let controller_parser = Arc::new(Mutex::new(ControllerParser::new()));
        let connection_manager = ConnectionManager::new(adapter.clone(), MAX_CONNECT_RETRIES.try_into().unwrap(), CONNECT_RETRY_DELAY_MS);
        let notification_handler = NotificationHandler::new(controller_parser.clone());

        Ok(Self {
            adapter,
            devices: Arc::new(Mutex::new(HashMap::new())),
            connected_state: Arc::new(Mutex::new(None)),
            connection_manager,
            notification_handler,
        })
    }

    fn extract_mac_address(device_id_str: &str) -> Option<String> {
        let re = Regex::new(r"([0-9A-Fa-f]{2}[:-]){5}([0-9A-Fa-f]{2})").unwrap();
        re.find(device_id_str).map(|mat| mat.as_str().to_string().to_uppercase())
    }
    
    /// Returns true if this device is a GearVR Controller
    fn is_gear_vr_controller(&self, device: &Device) -> bool {
        device.name()
            .ok()
            .as_ref()
            .map(|name| name.contains(CONTROLLER_NAME))
            .unwrap_or(false)
    }

    /// Emits a device-found event
    async fn emit_device_found(&self, window: Window, device: Device) -> Result<()>{
        let name = device.name().ok().or_else(|| Some("None".to_string()));
        let id = device.id().to_string();
        let rssi = device.rssi().await.ok().or_else(|| Some(0));
        let address = Self::extract_mac_address(&id).unwrap_or_else(|| "N/A".to_string());
        let is_paired = device.is_paired().await.unwrap_or(false);
        let is_connected = device.is_connected().await;
        let battery_level = None;
        
        let bluetooth_device = BluetoothDevice::new(
            id.clone(), name.clone(), Some(address.clone()), rssi,
            battery_level, Some(is_paired), Some(is_connected
        ));
        info!("Found Gear VR Controller device: Address: {}, ID: {}, Name: {:?}, RSSI: {:?}, 
            Battery Level: {:?}, Is Paired: {:?}, Is Connected: {:?}", 
            address, id, name, rssi, battery_level, is_paired, is_connected
        );

        {
            let mut devices: std::sync::MutexGuard<'_, HashMap<String, Device>> = self.devices.lock().unwrap();
            devices.insert(id.clone(), device.clone());
        }

        if let Err(e) = window.emit("device-found", bluetooth_device) {
            error!("Failed to emit device-found event: {}", e);
        }
        Ok(())
    }

    /// Scans for Bluetooth devices using bluest library
    pub async fn scan_devices_realtime(&self, window: Window, duration_secs: u64) -> Result<()> {
        // Clear existing devices
        self.devices.lock().unwrap().clear();

        // find connected device first
        info!("Checking for connected devices");
        let connected_devices = self.adapter.connected_devices().await?;
        for device in connected_devices {
            if self.is_gear_vr_controller(&device) {
                info!("Found connected Gear VR Controller device");
                self.emit_device_found(window.clone(), device).await?;
                return Ok(());
            }
        }
        info!("No connected Gear VR Controller detected");

        info!("Starting bluetooth scan for {} seconds", duration_secs);
        let mut scan = self.adapter.scan(&[]).await?;
        info!("Bluetooth scan started");

        // Process discovered peripherals in real-time
        let start_time = Instant::now();
        
        let scan_duration = Duration::from_secs(duration_secs);
        loop {
            let remaining_time = scan_duration.saturating_sub(start_time.elapsed());
            if remaining_time.is_zero() {
                info!("Scan duration of {} seconds has been reached.", duration_secs);
                break;
            }

            match timeout(remaining_time, scan.next()).await {
                Ok(Some(discovered_device)) => {
                    let device = discovered_device.device;
                    let rssi = discovered_device.rssi;
                    
                    // Print all discovered devices for debugging
                    debug!("Found device - Device: {:?}, RSSI: {:?}",  device, rssi);
                    // Only include devices with medium or stronger signal strength
                    if let Some(signal_strength) = rssi {
                        if signal_strength >= MIN_RSSI_THRESHOLD {
                            if self.is_gear_vr_controller(&device) {
                                self.emit_device_found(window.clone(), device).await?;
                                break;
                            }
                        }
                    }
                }

                Err(_) => {
                    info!("Scan timed out while waiting for a new device. Total duration reached.");
                    break;
                }

                Ok(None) => {
                    info!("Bluetooth scan stream has ended.");
                    break;
                }
            }
        }
        
        // Emit scan-complete event
        if let Err(e) = window.emit("scan-complete", ()) {
            error!("Failed to emit scan-complete event: {}", e);
        }
        Ok(())
    }

    /// Connects to a device with the given ID
    pub async fn connect_device(&self, device_id: &str, window: Window) -> Result<()> {
        let device = {
            let devices = self.devices.lock().unwrap();
            devices
                .get(device_id)
                .cloned()
                .ok_or_else(|| anyhow!("Device not found with ID: {}", device_id))?
        };

        info!("Connecting to device with ID: {}", device_id);
        
        // Connect to the device with retry mechanism
        let (notify_char, write_char) = self.connection_manager.connect_with_retry(
            &device,
            &window,
            &self.notification_handler,
            UUID_CONTROLLER_SERVICE,
            UUID_CONTROLLER_NOTIFY_CHAR,
            UUID_CONTROLLER_WRITE_CHAR,
        ).await?;
        
        let state = ConnectedDeviceState {
            device: device.clone(),
            notify_characteristic: notify_char,
            write_characteristic: write_char,
        };
        // If connection successful, store the connected device
        *self.connected_state.lock().unwrap() = Some(state);

        info!("Device successfully connected and state stored in the main service.");
        Ok(())
    }

    /// Disconnects from the currently connected device
    pub async fn disconnect(&self, device_id: &str) -> Result<()> {
        let device = {
            let devices = self.devices.lock().unwrap();
            devices
                .get(device_id)
                .cloned()
                .ok_or_else(|| anyhow!("Device not found with ID: {}", device_id))?
        };

        self.connection_manager.disconnect(&device).await?;
        Ok(())
    }

    /// turn off the controller
    pub async fn turn_off_controller(&self) -> Result<()> {
        let connected_state = {
            let connected = self.connected_state.lock().unwrap();
            connected.clone().ok_or_else(|| anyhow!("No device connected"))?
        };

        let command_sender = BluestCommandSender::new(connected_state.write_characteristic.clone());
        let command_executor = CommandExecutor::new(command_sender);

        command_executor.turn_off_controller().await
    }

    /// Calibrate the controller
    pub async fn calibrate_controller(&self) -> Result<()> {
        let connected_state = {
            let connected = self.connected_state.lock().unwrap();
            connected.clone().ok_or_else(|| anyhow!("No device connected"))?
        };
        
        // Find write characteristic
        let service = connected_state.device
            .discover_services_with_uuid(UUID_CONTROLLER_SERVICE).await?;
        
        let controller_service = service
            .first()
            .ok_or_else(|| anyhow!("Controller service not found"))?;

            let characteristics = controller_service.characteristics().await?;
        
        let write_char = characteristics
            .iter()
            .find(|c| c.uuid() == UUID_CONTROLLER_WRITE_CHAR)
            .ok_or_else(|| anyhow!("Write characteristic not found"))?;
        
        // Create command executor and send calibrate command
        let command_sender = BluestCommandSender::new(write_char.clone());
        let command_executor = CommandExecutor::new(command_sender);
        
        command_executor.calibrate_controller().await
    }

    /// Get battery level
    pub async fn get_battery_level(&self, device: &Device) -> Result<Option<u8>> {
        if !device.is_connected().await {
            info!("Device {:?} is not connected. Skipping battery level retrieval.", device.id());
            return Ok(None); // Return None if not connected
        }

        // Find battery service and characteristic
        let service = device
            .discover_services_with_uuid(UUID_BATTERY_SERVICE).await?;

        let battery_service = service
            .first()
            .ok_or_else(|| anyhow!("Battery service not found"))?;
        
        let characteristics = battery_service.characteristics().await?;
        let battery_char = characteristics
            .iter()
            .find(|c| c.uuid() == UUID_BATTERY_LEVEL)
            .ok_or_else(|| anyhow!("Battery level characteristic not found"))?;
        
        // Read battery level
        let battery_data = battery_char.read().await?;
        
        if battery_data.is_empty() {
            return Err(anyhow!("No battery level data received"));
        }

        Ok(Some(battery_data[0]))
    }

}