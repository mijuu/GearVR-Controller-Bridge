//! Bluetooth manager for the GearVR Controller Bridge
//! This module provides the main interface for bluetooth operations

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use bluest::{Adapter, Device};
use log::{info};
use tauri::{Window};

use crate::core::bluetooth::commands::CommandExecutor;
use crate::core::bluetooth::connection::{ConnectionManager, BluestCommandSender};
use crate::core::bluetooth::types::{ConnectedDeviceState};
use crate::core::bluetooth::scanner::{BluetoothScanner};
use crate::core::bluetooth::notification::NotificationHandler;
use crate::core::bluetooth::constants::{
    MAX_CONNECT_RETRIES,
    CONNECT_RETRY_DELAY_MS,
    UUID_CONTROLLER_SERVICE,
    UUID_CONTROLLER_NOTIFY_CHAR,
    UUID_CONTROLLER_WRITE_CHAR,
    UUID_BATTERY_SERVICE,
    UUID_BATTERY_LEVEL,
};
use crate::core::controller::ControllerParser;

/// Manages Bluetooth operations
pub struct BluetoothManager {
    /// The Bluetooth adapter
    // adapter: Adapter,
    /// Map of device addresses to devices
    devices: Arc<Mutex<HashMap<String, Device>>>,
    /// Currently connected device
    connected_state: Arc<Mutex<Option<ConnectedDeviceState>>>,
    /// Connection manager
    connection_manager: ConnectionManager,
    /// Bluetooth scanner
    scanner: BluetoothScanner,
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
        let devices = Arc::new(Mutex::new(HashMap::new()));

        let controller_parser = Arc::new(Mutex::new(ControllerParser::new()));
        let connection_manager = ConnectionManager::new(adapter.clone(), MAX_CONNECT_RETRIES.try_into().unwrap(), CONNECT_RETRY_DELAY_MS);
        let scanner = BluetoothScanner::new(adapter.clone(), devices.clone());
        let notification_handler = NotificationHandler::new(controller_parser.clone());

        Ok(Self {
            // adapter,
            devices,
            connected_state: Arc::new(Mutex::new(None)),
            connection_manager,
            scanner,
            notification_handler,
        })
    }

    /// Scans for Bluetooth devices using bluest library
    pub async fn start_scan(&mut self, window: Window) -> Result<()> {
        self.scanner.start_scan(window).await
    }

    pub async fn stop_scan(&mut self, window: Window) -> Result<()> {
        self.scanner.stop_scan(window).await
    }

    /// Connects to a device with the given ID
    pub async fn connect_device(&mut self, window: Window, device_id: &str) -> Result<()> {
        let device = {
            let devices = self.devices.lock().unwrap();
            devices
                .get(device_id)
                .cloned()
                .ok_or_else(|| anyhow!("Device not found with ID: {}", device_id))?
        };

        if device.is_connected().await {
            self.disconnect(window.clone(), device_id).await?;
        }
        
        // Connect to the device with retry mechanism
        let (notify_char, write_char) = self.connection_manager.connect_with_retry(
            &device,
            &window,
            &mut self.notification_handler,
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
    pub async fn disconnect(&mut self, window: Window, device_id: &str) -> Result<()> {
        let device = {
            let devices = self.devices.lock().unwrap();
            devices
                .get(device_id)
                .cloned()
                .ok_or_else(|| anyhow!("Device not found with ID: {}", device_id))?
        };

        self.notification_handler.abort_notifications();
        // drop ConnectedDeviceState
        {
            let mut connected_state_guard = self.connected_state.lock().unwrap();
            *connected_state_guard = None;
            info!("Connected state cleared, releasing device and characteristic objects.");
        }
        self.connection_manager.disconnect(window, &device).await?;

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