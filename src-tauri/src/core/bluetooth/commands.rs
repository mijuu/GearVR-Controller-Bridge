//! GearVR Controller commands implementation
//! This module contains all the commands that can be sent to the controller

use anyhow::Result;
use log::{debug, info, error};
use std::time::Duration;
use tokio::time::sleep;

/// Controller commands
#[derive(Debug, Clone, Copy)]
pub enum ControllerCommand {
    /// Turn off the controller (0x00, 0x00)
    Off,
    /// Enable sensor mode (0x01, 0x00)
    Sensor,
    /// Firmware update function (0x02, 0x00)
    FirmwareUpdate,
    /// Calibrate the controller (0x03, 0x00)
    Calibrate,
    /// Keep-alive signal (0x04, 0x00)
    KeepAlive,
    /// Unknown setting (0x05, 0x00)
    UnknownSetting,
    /// Enable LPM mode (0x06, 0x00)
    LpmEnable,
    /// Disable LPM mode (0x07, 0x00)
    LpmDisable,
    /// Enable VR mode (0x08, 0x00)
    VrMode,
}

impl ControllerCommand {
    /// Convert the command to its byte representation
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::Off => vec![0x00, 0x00],
            Self::Sensor => vec![0x01, 0x00],
            Self::FirmwareUpdate => vec![0x02, 0x00],
            Self::Calibrate => vec![0x03, 0x00],
            Self::KeepAlive => vec![0x04, 0x00],
            Self::UnknownSetting => vec![0x05, 0x00],
            Self::LpmEnable => vec![0x06, 0x00],
            Self::LpmDisable => vec![0x07, 0x00],
            Self::VrMode => vec![0x08, 0x00],
        }
    }
}

/// Command sender trait
#[async_trait::async_trait]
pub trait CommandSender {
    /// Send a command to the controller
    async fn send_command(&self, command: ControllerCommand) -> Result<()>;
}

/// Command executor for the controller
pub struct CommandExecutor<T: CommandSender> {
    command_sender: T,
}

impl<T: CommandSender> CommandExecutor<T> {
    /// Create a new CommandExecutor
    pub fn new(command_sender: T) -> Self {
        Self { command_sender }
    }

    pub async fn turn_off_controller(&self) -> Result<()> {
        info!("Turning off controller");
        self.command_sender.send_command(ControllerCommand::Off).await?;
        Ok(())
    }

    /// Initialize the controller with sensor mode or VR mode
    pub async fn initialize_controller(&self, vr_mode: bool) -> Result<()> {
        // disable LPM mode for smooth operation
        info!("Disabling LPM mode");
        self.command_sender.send_command(ControllerCommand::LpmDisable).await?;
        sleep(Duration::from_millis(100)).await;

        // Send the appropriate mode command
        let command = if vr_mode {
            info!("Setting VR Mode");
            ControllerCommand::VrMode
        } else {
            info!("Setting Sensor Mode");
            ControllerCommand::Sensor
        };

        self.command_sender.send_command(command).await?;
        
        // Wait for command to take effect
        sleep(Duration::from_millis(100)).await;
        info!("Controller initialized in {} mode", if vr_mode { "VR" } else { "Sensor" });

        Ok(())
    }

    /// Calibrate the controller
    pub async fn calibrate_controller(&self) -> Result<()> {
        info!("Starting controller calibration");
        self.command_sender.send_command(ControllerCommand::Calibrate).await?;
        sleep(Duration::from_millis(500)).await;
        info!("Controller calibration completed");
        Ok(())
    }

    /// Send a keep-alive signal
    pub async fn send_keepalive(&self) -> Result<()> {
        debug!("Sending keepalive signal");
        self.command_sender.send_command(ControllerCommand::KeepAlive).await?;
        Ok(())
    }

    /// Start the keepalive timer
    pub fn start_keepalive_timer(&self, interval_secs: u64)
    where
        T: Clone + Send + Sync + 'static,
    {
        let command_sender = self.command_sender.clone();
        
        tokio::spawn(async move {
            loop {
                if let Err(e) = command_sender.send_command(ControllerCommand::KeepAlive).await {
                    error!("Failed to send keepalive: {}", e);
                }
                sleep(Duration::from_secs(interval_secs)).await;
            }
        });
        
        info!("Keepalive timer started with interval of {} seconds", interval_secs);
    }
}