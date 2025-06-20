//! Notification handling for the GearVR Controller
//! This module handles setting up and processing notifications from the controller

use anyhow::{Result};
use bluest::{Characteristic};
use futures_util::StreamExt;
use log::{debug, error, info};
use std::sync::{Arc, Mutex};
use tauri::{Window, Emitter};

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
        notify_char: Characteristic,
        window: Window,
    ) -> Result<()> {
        info!("Subscribing to notifications...");
        
        // Clone necessary values for the async task
        let controller_parser = self.controller_parser.clone();
        
        // Start task to process notifications
        tokio::spawn(async move {
            Self::process_notifications(notify_char, controller_parser, window).await;
        });

        Ok(())
    }

    /// Process notifications from the controller
    async fn process_notifications(
        notify_char: Characteristic,
        controller_parser: Arc<Mutex<ControllerParser>>,
        window: Window,
    ) {
        info!("Listening for controller notifications...");
        
        match notify_char.notify().await {
            Ok(mut notification_stream) => {
                while let Some(result) = notification_stream.next().await {
                    match result {
                        Ok(value) => {
                            debug!("Received controller data: {:?}", value);

                            if value.len() < 10 {
                                error!("Unknow data: {:?}", value);
                                break;
                            }

                            // Parse the controller data
                            let controller_state = {
                                let mut parser = controller_parser.lock().unwrap();
                                parser.parse_data(&value)
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
                                            "uuid": notify_char.uuid().to_string(),
                                            "data": value,
                                        }),
                                    ) {
                                        error!("Failed to emit controller data: {}", e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Error in notification stream: {}", e);
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to subscribe to notifications: {}", e);
            }
        }

        info!("Notification stream ended");
    }
}