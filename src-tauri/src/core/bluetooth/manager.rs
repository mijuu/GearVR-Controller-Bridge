//! Bluetooth manager for the GearVR Controller Bridge
//! This module provides the main interface for bluetooth operations

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

use anyhow::{Result, anyhow};
use bluest::{Adapter, Device};
use log::{error, info};
use tauri::{Emitter, Manager, Window};
use tokio::sync::Mutex;

use crate::config::controller_config::ControllerConfig;
use crate::core::bluetooth::commands::CommandExecutor;
use crate::core::bluetooth::connection::{BluestCommandSender, ConnectionManager};
use crate::core::bluetooth::constants::{
    CONNECT_RETRY_DELAY_MS, MAX_CONNECT_RETRIES, UUID_BATTERY_LEVEL, UUID_BATTERY_SERVICE,
    UUID_CONTROLLER_NOTIFY_CHAR, UUID_CONTROLLER_SERVICE, UUID_CONTROLLER_WRITE_CHAR,
};
use crate::core::bluetooth::notification::NotificationHandler;
use crate::core::bluetooth::scanner::BluetoothScanner;
use crate::core::bluetooth::types::ConnectedDeviceState;
use crate::core::controller::ControllerParser;
use crate::mapping::mouse::MouseMapperSender;
use crate::utils::ensure_directory_exists;

/// Manages Bluetooth operations
pub struct BluetoothManager {
    /// The Bluetooth adapter
    adapter: Adapter,
    /// Map of device addresses to devices
    devices: Arc<Mutex<HashMap<String, Device>>>,
    /// Currently connected device
    connected_state: Arc<Mutex<Option<ConnectedDeviceState>>>,
    /// Connection manager
    connection_manager: ConnectionManager,
    /// Bluetooth scanner
    scanner: BluetoothScanner,
    /// Notification handler
    pub notification_handler: NotificationHandler,
}

impl BluetoothManager {
    /// Creates a new BluetoothManager
    pub async fn new(config: ControllerConfig) -> Result<Self> {
        let adapter = Adapter::default()
            .await
            .ok_or_else(|| anyhow!("No Bluetooth adapter found"))?;
        adapter.wait_available().await?;
        info!("Bluetooth adapter is available.");
        let devices = Arc::new(Mutex::new(HashMap::new()));

        let controller_parser = Arc::new(Mutex::new(ControllerParser::new(config)));
        let connection_manager = ConnectionManager::new(
            adapter.clone(),
            MAX_CONNECT_RETRIES.try_into().unwrap(),
            CONNECT_RETRY_DELAY_MS,
        );
        let scanner = BluetoothScanner::new(adapter.clone(), devices.clone());
        let notification_handler = NotificationHandler::new(controller_parser.clone());

        Ok(Self {
            adapter,
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
    pub async fn connect_device(
        &mut self,
        window: Window,
        device_id: &str,
        mouse_sender: MouseMapperSender,
    ) -> Result<()> {
        let device = {
            let devices = self.devices.lock().await;
            devices
                .get(device_id)
                .cloned()
                .ok_or_else(|| anyhow!("Device not found with ID: {}", device_id))?
        };

        // Connect to the device with retry mechanism
        let (notify_char, write_char, battery_char) = self
            .connection_manager
            .connect_with_retry(
                &device,
                &window,
                &mut self.notification_handler,
                mouse_sender.clone(),
                UUID_CONTROLLER_SERVICE,
                UUID_BATTERY_SERVICE,
                UUID_CONTROLLER_NOTIFY_CHAR,
                UUID_CONTROLLER_WRITE_CHAR,
                UUID_BATTERY_LEVEL,
            )
            .await?;

        let state = ConnectedDeviceState {
            device: device.clone(),
            mouse_sender,
            notify_characteristic: notify_char,
            write_characteristic: write_char,
            battery_characteristic: battery_char,
        };
        // If connection successful, store the connected device
        *self.connected_state.lock().await = Some(state);

        info!("Device successfully connected and state stored in the main service.");
        Ok(())
    }

    /// Reconnects to the last connected device
    pub async fn reconnect_device(&mut self, window: Window) -> Result<()> {
        let connected_state = {
            let connected_state_guard = self.connected_state.lock().await;
            connected_state_guard
                .clone()
                .ok_or_else(|| anyhow!("No device connected"))?
        };
        let last_device_id = connected_state.device.id();
        let mouse_sender = connected_state.mouse_sender;

        info!("Attempting to reconnect to device {}", &last_device_id);
        let device = self.adapter.open_device(&last_device_id).await?;

        // Connect to the device with retry mechanism
        let (notify_char, write_char, battery_char) = self
            .connection_manager
            .try_connect(
                &device,
                &window,
                &mut self.notification_handler,
                mouse_sender.clone(),
                UUID_CONTROLLER_SERVICE,
                UUID_BATTERY_SERVICE,
                UUID_CONTROLLER_NOTIFY_CHAR,
                UUID_CONTROLLER_WRITE_CHAR,
                UUID_BATTERY_LEVEL,
            )
            .await?;

        let state = ConnectedDeviceState {
            device: device.clone(),
            mouse_sender,
            notify_characteristic: notify_char,
            write_characteristic: write_char,
            battery_characteristic: battery_char,
        };
        // If connection successful, store the connected device
        *self.connected_state.lock().await = Some(state);

        info!("Device successfully reconnected and state stored in the main service.");
        Ok(())
    }

    /// Disconnects from the currently connected device
    pub async fn disconnect(&mut self) -> Result<()> {
        let connected_state = {
            let connected_state_guard = self.connected_state.lock().await;
            connected_state_guard
                .clone()
                .ok_or_else(|| anyhow!("No device connected"))?
        };

        let device = connected_state.device.clone();

        self.notification_handler.stop_notifications().await?;
        // drop ConnectedDeviceState
        {
            let mut connected_state_guard = self.connected_state.lock().await;
            *connected_state_guard = None;
            info!("Connected state cleared, releasing device and characteristic objects.");
        }
        self.connection_manager.disconnect(&device).await?;

        Ok(())
    }

    /// Checks if a device is currently connected.
    pub async fn is_connected(&self) -> bool {
        let guard = self.connected_state.lock().await;
        if let Some(state) = guard.as_ref() {
            state.device.is_connected().await
        } else {
            false
        }
    }

    /// Returns the ID of the currently connected device
    pub async fn get_connected_device_id(&self) -> Option<String> {
        let connected_state_guard = self.connected_state.lock().await;
        connected_state_guard
            .as_ref()
            .map(|state| state.device.id().to_string())
    }

    /// Returns the name of the currently connected device.
    pub async fn get_connected_device_name(&self) -> Option<String> {
        let guard = self.connected_state.lock().await;
        if let Some(state) = guard.as_ref() {
            let device = state.device.clone();
            drop(guard);
            match device.name() {
                Ok(name) => Some(name),
                Err(_) => None,
            }
        } else {
            None
        }
    }

    /// turn off the controller
    pub async fn turn_off_controller(&self) -> Result<()> {
        let connected_state = {
            let connected_state_guard = self.connected_state.lock().await;
            connected_state_guard
                .clone()
                .ok_or_else(|| anyhow!("No device connected"))?
        };

        let command_sender = BluestCommandSender::new(connected_state.write_characteristic.clone());
        let command_executor = CommandExecutor::new(command_sender);

        command_executor.turn_off_controller().await
    }

    /// Get battery level
    pub async fn get_battery_level(&mut self, window: Window) -> Result<Option<u8>> {
        let connected_state = {
            let connected_state_guard = self.connected_state.lock().await;
            connected_state_guard
                .clone()
                .ok_or_else(|| anyhow!("No device connected"))?
        };

        let device = connected_state.device.clone();
        if !device.is_connected().await {
            info!(
                "Device {:?} is not connected. Skipping battery level retrieval.",
                device.id()
            );

            if let Err(e) = window.emit("device-lost-connection", ()) {
                error!("Failed to emit device-lost-connection event: {}", e);
            }
            return Ok(None); // Return None if not connected
        }

        // Read battery level
        let battery_data = connected_state.battery_characteristic.read().await?;

        if battery_data.is_empty() {
            return Err(anyhow!("No battery level data received"));
        }

        Ok(Some(battery_data[0]))
    }

    /// Starts the calibration wizard.
    pub async fn start_mag_calibration_wizard(&self, window: Window) -> Result<()> {
        // Step 1: Prepare for calibration
        window.emit(
            "mag-calibration-step",
            "settings.calibration.mag.steps.starting",
        )?;
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");

        let cache_dir = window.app_handle().path().app_config_dir()?;
        let mut file_path = cache_dir.join("calibration_data");
        ensure_directory_exists(&file_path).await?;

        let file_name = format!("mag_calibration_data_{}.csv", timestamp);
        file_path.push(file_name);

        // Clear any previously recorded data for mag
        let controller_parser_arc = self.notification_handler.get_controller_parser();
        let mut controller_parser = controller_parser_arc.lock().await;
        controller_parser.clear_recorded_data(true, false).await; // Clear only mag data
        drop(controller_parser); // Drop the guard to release the lock

        // Step 2: Magnetometer calibration data collection (movement)
        window.emit(
            "mag-calibration-step",
            "settings.calibration.mag.steps.figure_eight",
        )?;
        self.start_calibration_recording(file_path.as_path())
            .await?;
        sleep(Duration::from_secs(10)).await; // Duration for figure-eight

        window.emit(
            "mag-calibration-step",
            "settings.calibration.mag.steps.tilt",
        )?;
        sleep(Duration::from_secs(8)).await;

        window.emit(
            "mag-calibration-step",
            "settings.calibration.mag.steps.rotate",
        )?;
        sleep(Duration::from_secs(8)).await;

        self.stop_calibration_recording().await?;
        window.emit(
            "mag-calibration-step",
            "settings.calibration.mag.steps.collection_complete",
        )?;

        // Perform magnetometer calibration
        match self.perform_mag_calibration().await {
            Ok(_) => {}
            Err(e) => {
                error!("Magnetometer calibration failed: {}", e);
                window.emit(
                    "mag-calibration-step",
                    "settings.calibration.mag.steps.failed",
                )?;
                window.emit("mag-calibration-finished", false)?;
                return Ok(());
            }
        }

        self.save_controller_config(window.clone()).await?;
        window.emit(
            "mag-calibration-step",
            "settings.calibration.mag.steps.success",
        )?;
        window.emit("mag-calibration-finished", true)?;

        Ok(())
    }

    pub async fn start_gyro_calibration(&self, window: Window) -> Result<()> {
        // Step 1: Prepare for calibration
        window.emit(
            "gyro-calibration-step",
            "settings.calibration.gyro.steps.starting",
        )?;
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");

        let cache_dir = window.app_handle().path().app_config_dir()?;
        let mut file_path = cache_dir.join("calibration_data");
        ensure_directory_exists(&file_path).await?;

        let file_name = format!("gyro_calibration_data_{}.csv", timestamp);
        file_path.push(file_name);

        // Clear any previously recorded data for gyro
        let controller_parser_arc = self.notification_handler.get_controller_parser();
        let mut controller_parser = controller_parser_arc.lock().await;
        controller_parser.clear_recorded_data(false, true).await; // Clear only gyro data
        drop(controller_parser); // Drop the guard to release the lock

        // Step 2: Gyroscope calibration data collection (stillness)
        window.emit(
            "gyro-calibration-step",
            "settings.calibration.gyro.steps.still",
        )?;
        self.start_calibration_recording(file_path.as_path())
            .await?;
        sleep(Duration::from_secs(5)).await; // Duration for stillness
        self.stop_calibration_recording().await?;
        window.emit(
            "gyro-calibration-step",
            "settings.calibration.gyro.steps.collection_complete",
        )?;

        // Perform gyroscope calibration
        match self.perform_gyro_calibration().await {
            Ok(_) => {}
            Err(e) => {
                error!("Gyroscope calibration failed: {}", e);
                window.emit(
                    "gyro-calibration-step",
                    "settings.calibration.gyro.steps.failed",
                )?;
                window.emit("gyro-calibration-finished", false)?;
                return Ok(());
            }
        }

        self.save_controller_config(window.clone()).await?;
        window.emit(
            "gyro-calibration-step",
            "settings.calibration.gyro.steps.success",
        )?;
        window.emit("gyro-calibration-finished", true)?;

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
        controller_parser
            .config
            .save_config(window.app_handle())
            .await
    }
}
