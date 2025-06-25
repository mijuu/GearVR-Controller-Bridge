
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use regex::Regex;
use anyhow::{Result};
use futures_util::StreamExt;
use bluest::{Adapter, Device};
use log::{info, debug, error};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tauri::{Window, Emitter};

use crate::core::bluetooth::types::{BluetoothDevice,};
use crate::core::bluetooth::constants::{
    CONTROLLER_NAME,
    MIN_RSSI_THRESHOLD,
};

pub struct BluetoothScanner {
    adapter: Adapter,
    devices: Arc<Mutex<HashMap<String, Device>>>,
    cancel_token: Arc<CancellationToken>,
    scan_task_handle: Option<JoinHandle<Result<()>>>,
}
impl BluetoothScanner {
    pub fn new(adapter: Adapter, devices: Arc<Mutex<HashMap<String, Device>>>) -> Self {
        Self {
            adapter,
            devices,
            cancel_token: Arc::new(CancellationToken::new()),
            scan_task_handle: None,
        }
    }
    pub async fn start_scan(&mut self, window: Window) -> Result<()> {
        // Clear existing devices
        self.devices.lock().unwrap().clear();
        if self.scan_task_handle.is_some() {
            self.stop_scan(window.clone()).await?;
        }

        self.cancel_token = Arc::new(CancellationToken::new());
        let cancel_token_for_task = self.cancel_token.clone();

        let adapter_for_task = self.adapter.clone();
        let devices_for_task = self.devices.clone();
        let window_for_task = window.clone();
        let min_rssi_threshold = MIN_RSSI_THRESHOLD;

        let handle = tokio::spawn(async move {
            let _ = Self::internal_scan_task(
                adapter_for_task,
                devices_for_task,
                window_for_task,
                cancel_token_for_task,
                min_rssi_threshold,
            ).await;
            Ok(())
        });

        self.scan_task_handle = Some(handle);

        // Emit scan-start event
        if let Err(e) = window.emit("scan-start", ()) {
            error!("Failed to emit scan-start event: {}", e);
        }
        info!("Device scan task started.");
        Ok(())
    }

    /// Scans for Bluetooth devices using bluest library
    async fn internal_scan_task(
        adapter: Adapter,
        devices: Arc<Mutex<HashMap<String, Device>>>,
        window: Window, cancel_token:
        Arc<CancellationToken>,
        min_rssi_threshold: i16
    ) -> Result<()> {
        // find connected device first
        info!("Checking for connected devices");
        let connected_devices = adapter.connected_devices().await?;
        for device in connected_devices {
            if BluetoothScanner::is_gear_vr_controller(&device) {
                BluetoothScanner::emit_device_found(devices, window.clone(), device).await?;
                if let Err(e) = window.emit("scan-complete", ()) {
                    error!("Failed to emit scan-complete event: {}", e);
                }
                return Ok(());
            }
        }
        info!("No connected Gear VR Controller detected");

        info!("Starting bluetooth scan");
        let mut scan_stream  = adapter.scan(&[]).await?;

        
        // Process discovered devices in real-time
        loop {
            tokio::select! {
                result = scan_stream .next() => {
                    match result {
                        Some(discovered_device) => {
                            let device = discovered_device.device;
                            let rssi = discovered_device.rssi;
                            
                            // Print all discovered devices for debugging
                            debug!("Found device - Device: {:?}, RSSI: {:?}",  device, rssi);
                            // Only include devices with medium or stronger signal strength
                            if let Some(signal_strength) = rssi {
                                if signal_strength >= min_rssi_threshold {
                                    if BluetoothScanner::is_gear_vr_controller(&device) {
                                        BluetoothScanner::emit_device_found(devices, window.clone(), device).await?;
                                        break;
                                    }
                                }
                            }
                        }
                        None => {
                            info!("Bluetooth scan stream has ended.");
                            break;
                        }
                    }
                }
                _ = cancel_token.cancelled() => {
                    break;
                }
            }
        }

        // Emit scan-complete event
        if let Err(e) = window.emit("scan-complete", ()) {
            error!("Failed to emit scan-complete event: {}", e);
        }
        Ok(())
    }

    pub async fn stop_scan(&mut self, window: Window, ) -> Result<()> {
        info!("Stopping Bluetooth scan.");
        self.cancel_token.cancel();

                // 等待任务结束
        if let Some(handle) = self.scan_task_handle.take() {
            info!("Waiting for scan task to finish...");
            // handle.await 会等待任务完成或被取消，并返回 JoinError 或任务的 Result
            
            match handle.await {
                Ok(task_result) => {
                    match task_result {
                        Ok(_) => info!("Scan task finished successfully after cancellation."),
                        Err(e) => error!("Scan task finished with an error: {:?}", e),
                    }
                },
                Err(e) => {
                    if e.is_cancelled() {
                        info!("Scan task was cancelled successfully.");
                    } else {
                        error!("Scan task finished with an unexpected join error: {:?}", e);
                    }
                }
            }
        } else {
            info!("No active scan task handle found to wait for.");
        }

        if let Err(e) = window.emit("stop-scan-complete", ()) {
            error!("Failed to emit stop-scan-complete event: {}", e);
        }
        Ok(())
    }

    /// Emits a device-found event
    async fn emit_device_found(devices: Arc<Mutex<HashMap<String, Device>>>, window: Window, device: Device) -> Result<()>{
        let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        let id = device.id().to_string();
        let rssi = device.rssi().await.unwrap_or_else(|_| 0);
        let address = Self::extract_mac_address(&id).unwrap_or_else(|| "N/A".to_string());
        let is_paired = device.is_paired().await.unwrap_or(false);
        let is_connected = device.is_connected().await;
        let battery_level = 0;
        
        let bluetooth_device = BluetoothDevice::new(
            id.clone(), name.clone(), address.clone(), rssi,
            battery_level, is_paired, is_connected
        );
        info!("Found Gear VR Controller device: Address: {}, ID: {}, Name: {:?}, RSSI: {:?}, 
            Battery Level: {:?}, Is Paired: {:?}, Is Connected: {:?}", 
            address, id, name, rssi, battery_level, is_paired, is_connected
        );

        {
            let mut devices: std::sync::MutexGuard<'_, HashMap<String, Device>> = devices.lock().unwrap();
            devices.insert(id.clone(), device.clone());
        }

        if let Err(e) = window.emit("device-found", bluetooth_device) {
            error!("Failed to emit device-found event: {}", e);
        }
        Ok(())
    }

    fn extract_mac_address(device_id_str: &str) -> Option<String> {
        let re = Regex::new(r"([0-9A-Fa-f]{2}[:-]){5}([0-9A-Fa-f]{2})").unwrap();
        re.find_iter(device_id_str).last().map(|m| m.as_str().to_string().to_uppercase())
    }
    
    /// Returns true if this device is a GearVR Controller
    fn is_gear_vr_controller(device: &Device) -> bool {
        device.name()
            .ok()
            .as_ref()
            .map(|name| name.contains(CONTROLLER_NAME))
            .unwrap_or(false)
    }
}