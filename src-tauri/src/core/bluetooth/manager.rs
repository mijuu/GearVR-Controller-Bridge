//! Bluetooth manager for the GearVR Controller Bridge
//! This module provides the main interface for bluetooth operations

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{anyhow, Result};
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};
use log::info;
use tauri::Window;

use crate::core::bluetooth::commands::CommandExecutor;
use crate::core::bluetooth::connection::{ConnectionManager, PeripheralCommandSender};
use crate::core::bluetooth::device::BluetoothDevice;
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
};
use crate::core::controller::ControllerParser;

/// Manages Bluetooth operations
#[derive(Clone)]
pub struct BluetoothManager {
    /// The Bluetooth adapter
    adapter: Adapter,
    /// Currently connected peripheral
    connected_peripheral: Arc<Mutex<Option<Peripheral>>>,
    /// Map of device addresses to peripherals
    peripherals: Arc<Mutex<HashMap<String, Peripheral>>>,
    /// Controller data parser
    controller_parser: Arc<Mutex<ControllerParser>>,
    /// Connection manager
    connection_manager: ConnectionManager,
    /// Notification handler
    notification_handler: NotificationHandler,
}

impl BluetoothManager {
    /// Creates a new BluetoothManager
    pub async fn new() -> Result<Self> {
        let manager = Manager::new().await?;
        let adapters = manager.adapters().await?;
        let adapter = adapters
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No Bluetooth adapters found"))?;

        info!("Using adapter: {}", adapter.adapter_info().await?);

        let controller_parser = Arc::new(Mutex::new(ControllerParser::new()));
        let connection_manager = ConnectionManager::new(MAX_CONNECT_RETRIES.try_into().unwrap(), CONNECT_RETRY_DELAY_MS);
        let notification_handler = NotificationHandler::new(controller_parser.clone());

        Ok(Self {
            adapter,
            connected_peripheral: Arc::new(Mutex::new(None)),
            peripherals: Arc::new(Mutex::new(HashMap::new())),
            controller_parser,
            connection_manager,
            notification_handler,
        })
    }

    /// Scans for Bluetooth devices
    pub async fn scan_devices(&self, duration_secs: u64) -> Result<Vec<BluetoothDevice>> {
        // Clear existing peripherals
        self.peripherals.lock().unwrap().clear();

        // Start scanning
        info!("Starting Bluetooth scan for {} seconds", duration_secs);
        self.adapter.start_scan(ScanFilter::default()).await?;

        // Wait for the specified duration
        tokio::time::sleep(Duration::from_secs(duration_secs)).await;

        // Stop scanning
        self.adapter.stop_scan().await?;
        info!("Bluetooth scan completed");

        // Get discovered peripherals
        let peripherals = self.adapter.peripherals().await?;
        let mut devices = Vec::new();

        // Process discovered peripherals
        for peripheral in peripherals {
            let properties = peripheral.properties().await?;
            let address = peripheral.address().to_string();

            // Only include devices with names and filter for Gear VR Controller
            if let Some(properties) = properties {
                let name = properties.local_name.clone();
                let rssi = properties.rssi;
                
                // Print all discovered devices for debugging
                info!("Found device - Address: {}, Name: {:?}, RSSI: {:?}", address, name, rssi);
                
                // Create device object
                let device = BluetoothDevice::new(name.clone(), address.clone(), rssi);
                
                // Only include Gear VR Controllers
                if device.is_gear_vr_controller(CONTROLLER_NAME) {
                    info!("Including Gear VR Controller device: {:?}", name);
                    
                    // Store peripheral for later connection
                    self.peripherals
                        .lock()
                        .unwrap()
                        .insert(address.clone(), peripheral);

                    devices.push(device);
                }
            }
        }

        Ok(devices)
    }

    /// Connects to a device with the given address
    pub async fn connect_device(&self, address: &str, window: Window) -> Result<()> {
        let peripheral = {
            let peripherals = self.peripherals.lock().unwrap();
            peripherals
                .get(address)
                .cloned()
                .ok_or_else(|| anyhow!("Device not found: {}", address))?
        };

        info!("Connecting to device: {}", address);
        
        // Connect to the device with retry mechanism
        let result = self.connection_manager.connect_with_retry(
            &peripheral,
            &window,
            &self.notification_handler,
            UUID_CONTROLLER_SERVICE,
            UUID_CONTROLLER_NOTIFY_CHAR,
            UUID_CONTROLLER_WRITE_CHAR,
        ).await;
        
        // If connection successful, store the connected peripheral
        if result.is_ok() {
            let mut connected = self.connected_peripheral.lock().unwrap();
            *connected = Some(peripheral);
            info!("Peripheral stored in connected_peripheral");
        }
        
        result
    }

    /// Disconnects from the currently connected device
    pub async fn disconnect(&self) -> Result<()> {
        let peripheral = {
            let mut connected = self.connected_peripheral.lock().unwrap();
            connected.take()
        };

        if let Some(peripheral) = peripheral {
            self.connection_manager.disconnect(&peripheral).await?;
        } else {
            info!("No device connected");
        }

        Ok(())
    }

    /// Calibrate the controller
    pub async fn calibrate_controller(&self) -> Result<()> {
        let peripheral = {
            let connected = self.connected_peripheral.lock().unwrap();
            connected.clone().ok_or_else(|| anyhow!("No device connected"))?
        };
        
        // Find write characteristic
        let services = peripheral.services();
        let controller_service = services
            .iter()
            .find(|s| s.uuid == UUID_CONTROLLER_SERVICE)
            .ok_or_else(|| anyhow!("Controller service not found"))?;
        
        let write_char = controller_service
            .characteristics
            .iter()
            .find(|c| c.uuid == UUID_CONTROLLER_WRITE_CHAR)
            .ok_or_else(|| anyhow!("Write characteristic not found"))?;
        
        // Create command executor and send calibrate command
        let command_sender = PeripheralCommandSender::new(peripheral, write_char.clone());
        let command_executor = CommandExecutor::new(command_sender);
        
        command_executor.calibrate_controller().await
    }

    /// Get battery level
    pub async fn get_battery_level(&self) -> Result<u8> {
        let peripheral = {
            let connected = self.connected_peripheral.lock().unwrap();
            connected.clone().ok_or_else(|| anyhow!("No device connected"))?
        };
        
        // Find battery service and characteristic
        let services = peripheral.services();
        let battery_service = services
            .iter()
            .find(|s| s.uuid == UUID_BATTERY_SERVICE)
            .ok_or_else(|| anyhow!("Battery service not found"))?;
        
        let battery_char = battery_service
            .characteristics
            .iter()
            .find(|c| c.uuid == UUID_BATTERY_LEVEL)
            .ok_or_else(|| anyhow!("Battery level characteristic not found"))?;
        
        // Read battery level
        let battery_data = peripheral.read(battery_char).await?;
        
        if battery_data.is_empty() {
            return Err(anyhow!("No battery level data received"));
        }

        Ok(battery_data[0])
    }
}