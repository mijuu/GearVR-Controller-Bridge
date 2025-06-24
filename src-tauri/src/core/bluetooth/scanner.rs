
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};

use regex::Regex;
use anyhow::{Result};
use bluest::{Adapter, Device};
use futures_util::StreamExt;
use log::{info, debug, error};
use tokio::time::timeout;
use tauri::{Window, Emitter};

use crate::core::bluetooth::types::{BluetoothDevice,};
use crate::core::bluetooth::constants::{
    CONTROLLER_NAME,
    MIN_RSSI_THRESHOLD,
};

#[derive(Clone)]
pub struct BluetoothScanner {
    adapter: Adapter,
    devices: Arc<Mutex<HashMap<String, Device>>>,
}
impl BluetoothScanner {
    pub fn new(adapter: Adapter, devices: Arc<Mutex<HashMap<String, Device>>>) -> Self {
        Self {
            adapter,
            devices,
        }
    }

    /// Scans for Bluetooth devices using bluest library
    pub async fn scan_devices_realtime(&self, window: Window, duration_secs: u64) -> Result<()> { 
        // Emit scan-start event
        if let Err(e) = window.emit("scan-start", ()) {
            eprintln!("Failed to emit scan-start event: {}", e);
        }

        // Clear existing devices
        self.devices.lock().unwrap().clear();

        // find connected device first
        info!("Checking for connected devices");
        let connected_devices = self.adapter.connected_devices().await?;
        for device in connected_devices {
            if self.is_gear_vr_controller(&device) {
                self.emit_device_found(window.clone(), device).await?;
                if let Err(e) = window.emit("scan-complete", ()) {
                    error!("Failed to emit scan-complete event: {}", e);
                }
                return Ok(());
            }
        }
        info!("No connected Gear VR Controller detected");

        info!("Starting bluetooth scan for {} seconds", duration_secs);
        let mut scan = self.adapter.scan(&[]).await?;
        info!("Bluetooth scan started");

        
        // Process discovered peripherals in real-time
        let start_time = Instant::now();
        
        let scan_duration = Duration::from_secs(duration_secs);
        loop {
            let remaining_time = scan_duration.saturating_sub(start_time.elapsed());
            if remaining_time.is_zero() {
                info!("Scan duration of {} seconds has been reached.", duration_secs);
                break;
            }

            match timeout(remaining_time, scan.next()).await {
                Ok(Some(discovered_device)) => {
                    let device = discovered_device.device;
                    let rssi = discovered_device.rssi;
                    
                    // Print all discovered devices for debugging
                    debug!("Found device - Device: {:?}, RSSI: {:?}",  device, rssi);
                    // Only include devices with medium or stronger signal strength
                    if let Some(signal_strength) = rssi {
                        if signal_strength >= MIN_RSSI_THRESHOLD {
                            if self.is_gear_vr_controller(&device) {
                                self.emit_device_found(window.clone(), device).await?;
                                break;
                            }
                        }
                    }
                }

                Err(_) => {
                    info!("Scan timed out while waiting for a new device. Total duration reached.");
                    break;
                }

                Ok(None) => {
                    info!("Bluetooth scan stream has ended.");
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

    /// Emits a device-found event
    async fn emit_device_found(&self, window: Window, device: Device) -> Result<()>{
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
            let mut devices: std::sync::MutexGuard<'_, HashMap<String, Device>> = self.devices.lock().unwrap();
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
    fn is_gear_vr_controller(&self, device: &Device) -> bool {
        device.name()
            .ok()
            .as_ref()
            .map(|name| name.contains(CONTROLLER_NAME))
            .unwrap_or(false)
    }
}