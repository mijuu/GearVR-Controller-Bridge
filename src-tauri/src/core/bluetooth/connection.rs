//! Bluetooth connection handling for the GearVR Controller
//! This module handles connecting to and disconnecting from the controller

use anyhow::{anyhow, Result};
use btleplug::api::{Peripheral as _, WriteType};
use btleplug::platform::Peripheral;
use log::{info, warn};
use std::time::Duration;
use tauri::Window;
use uuid::Uuid;

use crate::core::bluetooth::commands::{CommandExecutor, CommandSender, ControllerCommand};
use crate::core::bluetooth::notification::NotificationHandler;

/// Connection manager for the controller
#[derive(Clone)]
pub struct ConnectionManager {
    /// Maximum number of connection retries
    max_retries: u32,
    /// Delay between connection retries (ms)
    retry_delay: u64,
}

impl ConnectionManager {
    /// Create a new ConnectionManager
    pub fn new(max_retries: u32, retry_delay: u64) -> Self {
        Self {
            max_retries,
            retry_delay,
        }
    }

    /// Connect to the controller with retry mechanism
    pub async fn connect_with_retry(
        &self,
        peripheral: &Peripheral,
        window: &Window,
        notification_handler: &NotificationHandler,
        controller_service_uuid: Uuid,
        notify_char_uuid: Uuid,
        write_char_uuid: Uuid,
    ) -> Result<()> {
        let mut retry_count = 0;
        let mut last_error = None;

        while retry_count < self.max_retries {
            match self.try_connect(
                peripheral,
                window,
                notification_handler,
                controller_service_uuid,
                notify_char_uuid,
                write_char_uuid,
            ).await {
                Ok(_) => {
                    info!("Successfully connected to device");
                    return Ok(());
                }
                Err(e) => {
                    warn!("Connection attempt {} failed: {}", retry_count + 1, e);
                    last_error = Some(e);

                    if retry_count < self.max_retries - 1 {
                        info!("Retrying connection in {} ms...", self.retry_delay);
                        tokio::time::sleep(Duration::from_millis(self.retry_delay)).await;
                    }
                }
            }
            retry_count += 1;
        }

        // If all retries failed, return the last error
        Err(last_error.unwrap_or_else(|| anyhow!("Failed to connect after {} attempts", self.max_retries)))
    }

    /// Try to connect to the controller
    async fn try_connect(
        &self,
        peripheral: &Peripheral,
        window: &Window,
        notification_handler: &NotificationHandler,
        controller_service_uuid: Uuid,
        notify_char_uuid: Uuid,
        write_char_uuid: Uuid,
    ) -> Result<()> {
        // Print device details
        if let Ok(properties) = peripheral.properties().await {
            if let Some(props) = properties {
                info!("Device properties:");
                info!("  Name: {:?}", props.local_name);
                info!("  Address: {}", peripheral.address());
                info!("  RSSI: {:?}", props.rssi);
            }
        }

        // If already connected, disconnect first and wait
        if peripheral.is_connected().await? {
            info!("Device already connected, disconnecting first...");
            peripheral.disconnect().await?;
            info!("Waiting after disconnect...");
            tokio::time::sleep(Duration::from_millis(2000)).await;
        }

        // Connect to the device
        info!("Initiating connection...");
        match peripheral.connect().await {
            Ok(_) => info!("Initial connection successful"),
            Err(e) => {
                warn!("Initial connection failed: {}", e);
                // If connection fails, wait and try again
                tokio::time::sleep(Duration::from_millis(1000)).await;
                info!("Retrying connection...");
                peripheral.connect().await?;
            }
        }

        // Wait for connection to stabilize
        info!("Waiting for connection to stabilize...");
        tokio::time::sleep(Duration::from_millis(2000)).await;

        // Verify connection status
        if !peripheral.is_connected().await? {
            return Err(anyhow!("Connection failed to establish after waiting"));
        }

        info!("Connection verified");

        // Discover services
        info!("Discovering services...");
        peripheral.discover_services().await?;

        // Wait for service discovery to complete
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Find controller service
        let services = peripheral.services();
        let controller_service = services
            .iter()
            .find(|s| s.uuid == controller_service_uuid)
            .ok_or_else(|| {
                // If specific service not found, log all available services for debugging
                for service in &services {
                    info!("Available service: {}", service.uuid);
                }
                anyhow!("Controller service not found: {}", controller_service_uuid)
            })?;

        info!("Found controller service: {}", controller_service.uuid);

        // Find notification characteristic
        let notify_char = controller_service
            .characteristics
            .iter()
            .find(|c| c.uuid == notify_char_uuid)
            .ok_or_else(|| {
                // If specific characteristic not found, log all available characteristics for debugging
                for char in &controller_service.characteristics {
                    info!("Available characteristic: {} with properties {:?}",
                          char.uuid, char.properties);
                }
                anyhow!("Notification characteristic not found: {}", notify_char_uuid)
            })?;

        info!("Found notification characteristic: {}", notify_char.uuid);

        // Find write characteristic
        let write_char = controller_service
            .characteristics
            .iter()
            .find(|c| c.uuid == write_char_uuid)
            .ok_or_else(|| anyhow!("Write characteristic not found: {}", write_char_uuid))?;

        info!("Found write characteristic: {}", write_char.uuid);

        // Create command executor
        let command_sender = PeripheralCommandSender::new(peripheral.clone(), write_char.clone());
        let command_executor = CommandExecutor::new(command_sender);

        // Initialize controller
        info!("Initializing controller...");
        match command_executor.initialize_controller().await {
            Ok(_) => info!("Controller initialized successfully"),
            Err(e) => {
                warn!("Controller initialization failed: {}", e);
                // If initialization fails, wait and try again
                tokio::time::sleep(Duration::from_millis(1000)).await;
                info!("Retrying controller initialization...");
                command_executor.initialize_controller().await?;
            }
        }

        // Wait for initialization to complete
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Set up notifications
        info!("Setting up notifications...");
        notification_handler.setup_notifications(
            peripheral.clone(),
            notify_char,
            window.clone(),
        ).await?;

        // Start keepalive timer
        info!("Starting keepalive timer...");
        command_executor.start_keepalive_timer(30); // 30 seconds interval

        info!("Connection and setup process completed successfully");
        Ok(())
    }

    /// Disconnect from the controller
    pub async fn disconnect(&self, peripheral: &Peripheral) -> Result<()> {
        if peripheral.is_connected().await? {
            info!("Disconnecting from device");
            peripheral.disconnect().await?;
            info!("Successfully disconnected");
        } else {
            info!("Device not connected");
        }

        Ok(())
    }
}

/// Implementation of CommandSender for Peripheral
#[derive(Clone)]
pub struct PeripheralCommandSender {
    peripheral: Peripheral,
    write_char: btleplug::api::Characteristic,
}

impl PeripheralCommandSender {
    /// Create a new PeripheralCommandSender
    pub fn new(peripheral: Peripheral, write_char: btleplug::api::Characteristic) -> Self {
        Self {
            peripheral,
            write_char,
        }
    }
}

#[async_trait::async_trait]
impl CommandSender for PeripheralCommandSender {
    async fn send_command(&self, command: ControllerCommand) -> Result<()> {
        let data = command.to_bytes();
        
        // Determine write type
        let write_type = if self.write_char.properties.contains(btleplug::api::CharPropFlags::WRITE) {
            WriteType::WithResponse
        } else {
            WriteType::WithoutResponse
        };
        
        // Send data
        info!("Sending command to controller: {:?}", command);
        self.peripheral.write(&self.write_char, &data, write_type).await?;
        
        Ok(())
    }
}