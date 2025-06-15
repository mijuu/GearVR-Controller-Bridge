//! GearVR Controller commands implementation
//! This module contains all the commands that can be sent to the controller

use anyhow::Result;
use log::{debug, info};
use std::time::Duration;
use tokio::time::sleep;

/// Controller commands
#[derive(Debug, Clone, Copy)]
pub enum ControllerCommand {
    /// Turn off the controller (0x00, 0x00)
    Off,
    /// Enable sensor mode (0x01, 0x00)
    Sensor,
    /// Enable VR mode (0x02, 0x00)
    VrMode,
    /// Calibrate the controller (0x03, 0x00)
    Calibrate,
    /// Keep-alive signal (0x04, 0x00)
    KeepAlive,
}

impl ControllerCommand {
    /// Convert the command to its byte representation
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::Off => vec![0x00, 0x00],
            Self::Sensor => vec![0x01, 0x00],
            Self::VrMode => vec![0x02, 0x00],
            Self::Calibrate => vec![0x03, 0x00],
            Self::KeepAlive => vec![0x04, 0x00],
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

    /// Initialize the controller with the required command sequence
    pub async fn initialize_controller(&self) -> Result<()> {
        // First send OFF command to reset controller state
        info!("Sending OFF command to reset controller state");
        self.command_sender.send_command(ControllerCommand::Off).await?;
        sleep(Duration::from_millis(500)).await;

        // Initialize command sequence
        let init_commands = [
            ControllerCommand::Sensor,
            ControllerCommand::VrMode,
            ControllerCommand::KeepAlive,
        ];

        // Send initialization commands
        for cmd in &init_commands {
            info!("Sending command: {:?}", cmd);
            self.command_sender.send_command(*cmd).await?;
            sleep(Duration::from_millis(500)).await;
        }

        // Wait for commands to take effect
        sleep(Duration::from_millis(1000)).await;
        info!("Controller initialized");

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
                    log::error!("Failed to send keepalive: {}", e);
                }
                sleep(Duration::from_secs(interval_secs)).await;
            }
        });
        
        info!("Keepalive timer started with interval of {} seconds", interval_secs);
    }
}