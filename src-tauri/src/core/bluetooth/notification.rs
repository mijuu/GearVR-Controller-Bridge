//! Notification handling for the GearVR Controller
//! This module handles setting up and processing notifications from the controller

use anyhow::{anyhow, Result};
use btleplug::api::{Characteristic, CharPropFlags, Peripheral as _};
use btleplug::platform::Peripheral;
use futures_util::stream::StreamExt;
use log::{debug, error, info};
use std::sync::{Arc, Mutex};
use tauri::{Window, Emitter};
use uuid::Uuid;

use crate::core::controller::ControllerParser;

/// Notification handler for controller data
#[derive(Clone)]
pub struct NotificationHandler {
    /// Controller data parser
    controller_parser: Arc<Mutex<ControllerParser>>,
}

impl NotificationHandler {
    /// Create a new NotificationHandler
    pub fn new(controller_parser: Arc<Mutex<ControllerParser>>) -> Self {
        Self { controller_parser }
    }

    /// Set up notifications for the controller
    pub async fn setup_notifications(
        &self,
        peripheral: Peripheral,
        notify_char: &Characteristic,
        window: Window,
    ) -> Result<()> {
        // Check if characteristic supports notifications
        if !notify_char.properties.contains(CharPropFlags::NOTIFY) {
            return Err(anyhow!("Characteristic does not support notifications"));
        }

        // Subscribe to notifications
        info!("Subscribing to notifications...");
        peripheral.subscribe(notify_char).await?;
        info!("Subscribed successfully");

        // Clone necessary values for the async task
        let peripheral_clone = peripheral.clone();
        let controller_parser = self.controller_parser.clone();
        let notify_uuid = notify_char.uuid;

        // Start task to process notifications
        tokio::spawn(async move {
            Self::process_notifications(peripheral_clone, notify_uuid, controller_parser, window).await;
        });

        Ok(())
    }

    /// Process notifications from the controller
    async fn process_notifications(
        peripheral: Peripheral,
        notify_uuid: Uuid,
        controller_parser: Arc<Mutex<ControllerParser>>,
        window: Window,
    ) {
        let mut notification_stream = match peripheral.notifications().await {
            Ok(stream) => stream,
            Err(e) => {
                error!("Failed to get notification stream: {}", e);
                return;
            }
        };

        info!("Listening for controller notifications...");

        while let Some(notification) = notification_stream.next().await {
            if notification.uuid == notify_uuid {
                debug!("Received controller data: {:?}", notification.value);

                // Parse the controller data
                let controller_state = {
                    let mut parser = controller_parser.lock().unwrap();
                    parser.parse_data(&notification.value)
                };

                match controller_state {
                    Some(state) => {
                        debug!("Parsed controller state: {:?}", state);

                        // Send parsed controller state to frontend
                        if let Err(e) = window.emit(
                            "controller-state",
                            serde_json::json!(state),
                        ) {
                            error!("Failed to emit controller state: {}", e);
                        }
                    }
                    None => {
                        error!("Failed to parse controller data: no valid state");

                        // Send raw data to frontend (for debugging)
                        if let Err(e) = window.emit(
                            "controller-data",
                            serde_json::json!({
                                "uuid": notification.uuid.to_string(),
                                "data": notification.value,
                            }),
                        ) {
                            error!("Failed to emit controller data: {}", e);
                        }
                    }
                }
            } else {
                debug!(
                    "Received notification from unexpected characteristic: {}",
                    notification.uuid
                );
            }
        }

        info!("Notification stream ended");
    }
}