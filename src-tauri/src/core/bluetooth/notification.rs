//! Notification handling for the GearVR Controller
//! This module handles setting up and processing notifications from the controller

use anyhow::{Result};
use bluest::{Characteristic};
use futures_util::StreamExt;
use log::{debug, error, info};
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tauri::{Window, Emitter};

use crate::core::controller::ControllerParser;

/// Notification handler for controller data
pub struct NotificationHandler {
    /// Controller data parser
    controller_parser: Arc<Mutex<ControllerParser>>,
    cancel_token: Arc<CancellationToken>,
    task_handle: Option<JoinHandle<Result<()>>>,
}

impl NotificationHandler {
    /// Create a new NotificationHandler
    pub fn new(controller_parser: Arc<Mutex<ControllerParser>>) -> Self {
        Self {
            controller_parser,
            cancel_token: Arc::new(CancellationToken::new()),
            task_handle: None,
        }
    }

    /// Set up notifications for the controller
    pub async fn setup_notifications(
        &mut self,
        window: Window,
        notify_char: Characteristic,
    ) -> Result<()> {
        if self.task_handle.is_some() {
            self.abort_notifications().await?;
        }
        self.cancel_token = Arc::new(CancellationToken::new());

        info!("Subscribing to notifications...");
        // Clone necessary values for the async task
        let controller_parser = self.controller_parser.clone();
        let cancel_token = self.cancel_token.clone();

        // Start task to process notifications
        tokio::spawn(async move {
            Self::process_notifications(window, notify_char, controller_parser, cancel_token).await;
        });

        Ok(())
    }

    /// Process notifications from the controller
    async fn process_notifications(
        window: Window,
        notify_char: Characteristic,
        controller_parser: Arc<Mutex<ControllerParser>>,
        cancel_token: Arc<CancellationToken>,
    ) {
        info!("Listening for controller notifications...");
        
        match notify_char.notify().await {
            Ok(mut notification_stream) => {
                loop {
                    tokio::select! {
                        result = notification_stream.next() => {
                            match result {
                                Some(Ok(value)) => {
                                    debug!("Received controller data: {:?}", value);

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
                                Some(Err(e)) => {
                                    error!("Error in notification stream: {}", e);
                                    break;
                                }
                                None => {
                                    info!("Notification stream ended gracefully (no more items).");
                                    break;
                                }
                            }
                        }
                        _ = cancel_token.cancelled() => {
                            info!("Notification processing cancelled by token.");
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

    pub async  fn abort_notifications(&mut self) -> Result<()> {
        info!("Aborting notification: Cancel signal sent.");
        self.cancel_token.cancel();

        // 等待任务结束
        if let Some(handle) = self.task_handle.take() {
            info!("Waiting for notification to finish...");
            // handle.await 会等待任务完成或被取消，并返回 JoinError 或任务的 Result
            
            match handle.await {
                Ok(task_result) => {
                    match task_result {
                        Ok(_) => info!("Notification finished successfully after cancellation."),
                        Err(e) => error!("Notification finished with an error: {:?}", e),
                    }
                },
                Err(e) => {
                    if e.is_cancelled() {
                        info!("Notification was cancelled successfully.");
                    } else {
                        error!("Notification finished with an unexpected join error: {:?}", e);
                    }
                }
            }
        } else {
            info!("No active notification handle found to wait for.");
        }
        Ok(())
    }
}