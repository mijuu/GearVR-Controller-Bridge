//! Bluetooth connection handling for the GearVR Controller
//! This module handles connecting to and disconnecting from the controller

use anyhow::{anyhow, Result};
use bluest::{Adapter, Characteristic, Device, Uuid};
use log::{info, warn, error};
use std::time::Duration;
use tauri::{Window, Emitter};

use crate::core::bluetooth::types::{BluetoothDevice};
use crate::core::bluetooth::notification::NotificationHandler;
use crate::core::bluetooth::{commands::{CommandExecutor, CommandSender, ControllerCommand}};

/// Connection manager for the controller
#[derive(Clone)]
pub struct ConnectionManager {
    adapter: Adapter,
    max_retries: u32,
    retry_delay: u64,
}

impl ConnectionManager {
    pub fn new(adapter: Adapter, max_retries: u32, retry_delay: u64) -> Self {
        Self {adapter, max_retries, retry_delay }
    }

    /// Connect to the controller with retry mechanism (bluest version)
    pub async fn connect_with_retry(
        &self,
        device: &Device,
        window: &Window,
        notification_handler: &NotificationHandler,
        controller_service_uuid: Uuid,
        notify_char_uuid: Uuid,
        write_char_uuid: Uuid,
    ) -> Result<(Characteristic, Characteristic)> {
        let mut retry_count = 0;
        let mut last_error = None;

        while retry_count < self.max_retries {
            match self.try_connect(
                device,
                window,
                notification_handler,
                controller_service_uuid,
                notify_char_uuid,
                write_char_uuid,
            ).await {
                Ok((notify_char, write_char)) => {
                    info!("Successfully connected to device");
                    return Ok((notify_char, write_char));
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

        Err(last_error.unwrap_or_else(|| anyhow!("Failed to connect after {} attempts", self.max_retries)))
    }

    /// Try to connect to the controller
    async fn try_connect(
        &self,
        device: &Device,
        window: &Window,
        notification_handler: &NotificationHandler,
        controller_service_uuid: Uuid,
        notify_char_uuid: Uuid,
        write_char_uuid: Uuid,
    ) -> Result<(Characteristic, Characteristic)> {
        let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        let id = device.id().to_string();
        info!("Device details - ID: {}, Name: {:?}", id, name);


        if !device.is_connected().await {
            info!("Initiating connection to {}...", id);
            self.adapter.connect_device(&device).await?;
        }
        
        
        info!("Connection successful, discovering services...");
        let services = device.services().await?;
        let controller_service = services
            .iter()
            .find(|s| s.uuid() == controller_service_uuid)
            .ok_or_else(|| {
                for service in &services {
                    info!("Available service: {}", service.uuid());
                }
                anyhow!("Controller service not found: {}", controller_service_uuid)
            })?.clone();

        info!("Found controller service: {}", controller_service.uuid());

        let mut notify_char_opt = None;
        let mut write_char_opt = None;

        for char in controller_service.characteristics().await? {
            let uuid = char.uuid();
            if uuid == notify_char_uuid {
                info!("Found notification characteristic: {}", uuid);
                notify_char_opt = Some(char.clone());
            } else if uuid == write_char_uuid {
                info!("Found write characteristic: {}", uuid);
                write_char_opt = Some(char.clone());
            }
        }

        let notify_char = notify_char_opt.ok_or_else(|| anyhow!("Notification characteristic not found: {}", notify_char_uuid))?;
        let write_char = write_char_opt.ok_or_else(|| anyhow!("Write characteristic not found: {}", write_char_uuid))?;

        // 创建新的 CommandSender
        let command_sender = BluestCommandSender::new(write_char.clone());
        let command_executor = CommandExecutor::new(command_sender);


        let notify_char_for_task = notify_char.clone();

        // 设置通知监听
        info!("Setting up notifications...");
        notification_handler.setup_notifications(
            notify_char_for_task,
            window.clone(),
        ).await?;

        info!("Initializing controller in sensor mode...");
        command_executor.initialize_controller(false).await?;
        info!("Controller initialized successfully in sensor mode");

        // info!("Starting keepalive timer...");
        // command_executor.start_keepalive_timer(60);

        info!("Connection and setup process completed successfully");
        let bluetooth_device = BluetoothDevice::new(id,Some(name), None, None, None, Some(true), Some(true));
        if let Err(e) = window.emit("update-device", bluetooth_device) {
            error!("Failed to emit update-device event: {}", e);
        }
        Ok((notify_char, write_char))
    }

    /// Disconnect from the controller (bluest version)
    pub async fn disconnect(&self, device: &Device) -> Result<()> {
        if device.is_connected().await {
            info!("Disconnecting from device {}", device.id());
            self.adapter.disconnect_device(device).await?;
            info!("Successfully disconnected");
        } else {
            info!("Device {} not connected", device.id());
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct BluestCommandSender {
    // 不再需要整个 device 对象，只需要写入特征
    write_char: bluest::Characteristic,
}

impl BluestCommandSender {
    pub fn new(write_char: bluest::Characteristic) -> Self {
        Self { write_char }
    }
}

#[async_trait::async_trait]
impl CommandSender for BluestCommandSender {
    async fn send_command(&self, command: ControllerCommand) -> Result<()> {
        let data = command.to_bytes();
        
        // bluest 的写入非常直接，在特征上调用 .write() 即可
        // 它不需要像 btleplug 那样手动指定 WriteType
        info!("Sending command to controller: {:?}", command);
        self.write_char.write(&data).await?;
        
        Ok(())
    }
}