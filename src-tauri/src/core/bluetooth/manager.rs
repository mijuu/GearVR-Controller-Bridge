//! Bluetooth manager for the GearVR Controller Bridge
//! This module provides the main interface for bluetooth operations

use std::collections::HashMap;
use std::sync::{Arc};
use std::path::Path;
use tokio::time::{sleep, Duration};

use anyhow::{anyhow, Result};
use bluest::{Adapter, Device};
use log::{info, error};
use tokio::sync::{Mutex};
use tauri::{Emitter, Manager, Window};

use crate::mapping::mouse::MouseMapperSender;
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
use crate::core::controller::{ControllerParser};
use crate::config::controller_config::ControllerConfig;
use crate::utils::ensure_directory_exists;

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
    pub async fn new(initial_config: Option<ControllerConfig>) -> Result<Self> {
        let adapter = Adapter::default().await
            .ok_or_else(|| anyhow!("No Bluetooth adapter found"))?;
        adapter.wait_available().await?;
        info!("Bluetooth adapter is available.");
        let devices = Arc::new(Mutex::new(HashMap::new()));

        let controller_parser = Arc::new(Mutex::new(ControllerParser::new(initial_config.unwrap_or_default())));
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
    pub async fn connect_device(&mut self, window: Window, device_id: &str, mouse_sender: MouseMapperSender,) -> Result<()> {
        let device = {
            let devices = self.devices.lock().await;
            devices
                .get(device_id)
                .cloned()
                .ok_or_else(|| anyhow!("Device not found with ID: {}", device_id))?
        };

        if device.is_connected().await {
            self.disconnect(window.clone(), device_id).await?;
        }

        if cfg!(target_os = "windows") {
            if device.is_paired().await? {
                info!("Device is already paired, unpairing...");
                device.unpair().await?;
            }
        }
        
        // Connect to the device with retry mechanism
        let (notify_char, write_char) = self.connection_manager.connect_with_retry(
            &device,
            &window,
            &mut self.notification_handler,
            mouse_sender,
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
        *self.connected_state.lock().await = Some(state);

        info!("Device successfully connected and state stored in the main service.");
        Ok(())
    }

    /// Disconnects from the currently connected device
    pub async fn disconnect(&mut self, window: Window, device_id: &str) -> Result<()> {
        let device = {
            let devices = self.devices.lock().await;
            devices
                .get(device_id)
                .cloned()
                .ok_or_else(|| anyhow!("Device not found with ID: {}", device_id))?
        };

        self.notification_handler.stop_notifications().await?;
        // drop ConnectedDeviceState
        {
            let mut connected_state_guard = self.connected_state.lock().await;
            *connected_state_guard = None;
            info!("Connected state cleared, releasing device and characteristic objects.");
        }
        self.connection_manager.disconnect(window, &device).await?;

        Ok(())
    }

    /// Returns the ID of the currently connected device
    pub async fn get_connected_device_id(&self) -> Option<String> {
    let connected_state_guard = self.connected_state.lock().await;
    connected_state_guard
        .as_ref()
        .map(|state| state.device.id().to_string())
    }

    /// turn off the controller
    pub async fn turn_off_controller(&self) -> Result<()> {
        let connected_state = {
            let connected_state_guard = self.connected_state.lock().await;
            connected_state_guard.clone().ok_or_else(|| anyhow!("No device connected"))?
        };

        let command_sender = BluestCommandSender::new(connected_state.write_characteristic.clone());
        let command_executor = CommandExecutor::new(command_sender);

        command_executor.turn_off_controller().await
    }

    /// Calibrate the controller
    pub async fn calibrate_controller(&self) -> Result<()> {
        let connected_state = {
            let connected_state_guard = self.connected_state.lock().await;
            connected_state_guard.clone().ok_or_else(|| anyhow!("No device connected"))?
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
    pub async fn get_battery_level(&self, window: Window) -> Result<Option<u8>> {
        let connected_state = {
            let connected_state_guard = self.connected_state.lock().await;
            connected_state_guard.clone().ok_or_else(|| anyhow!("No device connected"))?
        };

        let device = connected_state.device.clone();
        if !device.is_connected().await {
            info!("Device {:?} is not connected. Skipping battery level retrieval.", device.id());
            
            if let Err(e) = window.emit("device-lost-connection", ()) {
                error!("Failed to emit device-lost-connection event: {}", e);
            }
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

    /// Starts the calibration wizard.
    pub async fn start_calibration_wizard(&self, window: Window) -> Result<()> {
        // Step 1: Prepare for calibration
        window.emit("calibration-step", "Starting calibration... Please prepare to move the controller.")?;
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");

        let cache_dir = window.app_handle().path().app_config_dir()?;
        let mut file_path = cache_dir.join("calibration_data");
        ensure_directory_exists(&file_path).await?;

        let file_name = format!("sensor_data_{}.csv", timestamp);
        file_path.push(file_name);

        // Clear any previously recorded data
        let controller_parser_arc = self.notification_handler.get_controller_parser();
        let mut controller_parser = controller_parser_arc.lock().await;
        controller_parser.clear_recorded_data(true, true).await;
        drop(controller_parser);

        // Step 2: Magnetometer calibration data collection
        window.emit("calibration-step", "Slowly rotate the controller in a figure-eight pattern for magnetometer calibration.")?;
        sleep(Duration::from_secs(5)).await;
        self.start_calibration_recording(file_path.as_path()).await?;
        sleep(Duration::from_secs(10)).await;

        window.emit("calibration-step", "Slowly tilt the controller up and down.")?;
        sleep(Duration::from_secs(5)).await;

        window.emit("calibration-step", "Slowly rotate the controller horizontally.")?;
        sleep(Duration::from_secs(5)).await;

        self.stop_calibration_recording().await?;
        window.emit("calibration-step", "Magnetometer data collection complete. Performing calibration...")?;

        // Perform magnetometer calibration
        match self.perform_mag_calibration().await {
            Ok(_) => {
                window.emit("calibration-step", "Magnetometer calibration successful!")?;
            }
            Err(e) => {
                error!("Magnetometer calibration failed: {}", e);
                window.emit("calibration-step", "Magnetometer calibration failed. Please try again.")?;
                window.emit("calibration-finished", false)?;
                return Ok(());
            }
        }

        // Step 3: Gyroscope calibration data collection
        window.emit("calibration-step", "Please place the controller still on a flat surface for gyroscope calibration.")?;
        // Clear magnetometer data, but keep gyroscope data for now
        let controller_parser_arc = self.notification_handler.get_controller_parser();
        let mut controller_parser = controller_parser_arc.lock().await;
        controller_parser.clear_recorded_data(false, true).await;
        drop(controller_parser);

        sleep(Duration::from_secs(10)).await;
        // Re-start recording for gyroscope calibration
        self.start_calibration_recording(file_path.as_path()).await?;
        sleep(Duration::from_secs(5)).await;
        self.stop_calibration_recording().await?;
        window.emit("calibration-step", "Gyroscope data collection complete. Performing calibration...")?;

        // Perform gyroscope calibration
        match self.perform_gyro_calibration().await {
            Ok(_) => {
                window.emit("calibration-step", "Gyroscope calibration successful!")?;
            }
            Err(e) => {
                error!("Gyroscope calibration failed: {}", e);
                window.emit("calibration-step", "Gyroscope calibration failed. Please try again.")?;
                window.emit("calibration-finished", false)?;
                return Ok(());
            }
        }

        self.save_controller_config(window.clone()).await?;
        window.emit("calibration-step", "All calibrations successful!")?;
        window.emit("calibration-finished", true)?;

        Ok(())
    }

    /// Starts recording sensor data for calibration.
    async fn start_calibration_recording(&self, file_path: &Path) -> Result<()> {
        let controller_parser_arc = self.notification_handler.get_controller_parser();
        let mut controller_parser = controller_parser_arc.lock().await;
        controller_parser.start_data_recording(file_path);
        Ok(())
    }

    /// Stops recording sensor data for calibration.
    async fn stop_calibration_recording(&self) -> Result<()> {
        let controller_parser_arc = self.notification_handler.get_controller_parser();
        let mut controller_parser = controller_parser_arc.lock().await;
        controller_parser.stop_data_recording();
        Ok(())
    }

    /// Performs magnetometer calibration using recorded data.
    async fn perform_mag_calibration(&self) -> Result<()> {
        let controller_parser_arc = self.notification_handler.get_controller_parser();
        let mut controller_parser = controller_parser_arc.lock().await;
        controller_parser.perform_mag_calibration().await
    }

    /// Performs gyroscope calibration using recorded data.
    async fn perform_gyro_calibration(&self) -> Result<()> {
        let controller_parser_arc = self.notification_handler.get_controller_parser();
        let mut controller_parser = controller_parser_arc.lock().await;
        controller_parser.perform_gyro_calibration().await
    }

    /// Saves the current controller config to a configuration file.
    async fn save_controller_config(&self, window: Window) -> Result<()> {
        let controller_parser_arc = self.notification_handler.get_controller_parser();
        let mut controller_parser = controller_parser_arc.lock().await;
        eprintln!("Saving controller config...");
        // The config is now saved via the ControllerConfig struct
        controller_parser.config.save_config(window.app_handle()).await
    }
}