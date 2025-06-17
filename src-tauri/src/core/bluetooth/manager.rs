//! Bluetooth manager for the GearVR Controller Bridge
//! This module provides the main interface for bluetooth operations

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};

use regex::Regex;
use anyhow::{anyhow, Result};
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager as BtleplugManager, Peripheral};
use bluest::Adapter as BlueAdapter;
use futures_util::StreamExt;
use log::info;
use tokio::time::timeout;
use tauri::{Window, Emitter};

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
    MIN_RSSI_THRESHOLD,
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
        let manager = BtleplugManager::new().await?;
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

    fn extract_mac_address(device_id_str: &str) -> Option<String> {
        let re = Regex::new(r"([0-9A-Fa-f]{2}[:-]){5}([0-9A-Fa-f]{2})").unwrap();
        re.find(device_id_str).map(|mat| mat.as_str().to_string().to_uppercase())
    }
    /// Scans for Bluetooth devices using bluest library
    pub async fn scan_devices_realtime(&self, window: Window, duration_secs: u64) -> Result<()> {
        let adapter = BlueAdapter::default().await.ok_or_else(|| anyhow!("No Bluetooth adapter found"))?;
        adapter.wait_available().await?;
        
        // Clear existing peripherals
        self.peripherals.lock().unwrap().clear();

        info!("Starting bluetooth scan for {} seconds", duration_secs);
        let mut scan = adapter.scan(&[]).await?;
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
                    let name = discovered_device.adv_data.local_name;
                    let id = device.id().to_string();
                    let rssi = discovered_device.rssi;
                    let address = Self::extract_mac_address(&id).unwrap_or_else(|| "N/A".to_string());
                    
                    // Print all discovered devices for debugging
                    info!("Found device - Address: {}, ID: {}, Name: {:?}, RSSI: {:?}", 
                        address, id, name, rssi);

                    // Only include devices with medium or stronger signal strength
                    if let Some(signal_strength) = rssi {
                        if signal_strength >= MIN_RSSI_THRESHOLD {
                            info!("Including device with sufficient signal strength (RSSI: {})", signal_strength);
                            let bluetooth_device = BluetoothDevice::new(name.clone(), address, id, rssi);
                            if bluetooth_device.is_gear_vr_controller(CONTROLLER_NAME) {
                                info!("Including Gear VR Controller device: {:?}", name);
                                if let Err(e) = window.emit("device-found", bluetooth_device) {
                                    info!("Failed to emit device-found event: {}", e);
                                }
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
            info!("Failed to emit scan-complete event: {}", e);
        }
        Ok(())
    }

    /// Connects to a device with the given ID
    pub async fn connect_device(&self, device_id: &str, window: Window) -> Result<()> {
        let peripheral = {
            let peripherals = self.peripherals.lock().unwrap();
            peripherals
                .get(device_id)
                .cloned()
                .ok_or_else(|| anyhow!("Device not found with ID: {}", device_id))?
        };

        info!("Connecting to device with ID: {}", device_id);
        
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