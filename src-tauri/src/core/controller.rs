//! GearVR Controller data parsing and handling
//! This module handles parsing and processing of data received from the GearVR controller.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Represents the state of the GearVR controller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerState {
    /// Timestamp when this state was created
    pub timestamp: u64,
    
    /// Button states
    pub buttons: ButtonState,
    
    /// Touchpad state
    pub touchpad: TouchpadState,
    
    /// Orientation data (quaternion)
    pub orientation: Quaternion,
    
    /// Accelerometer data (in m/s²)
    pub accelerometer: Vector3,
    
    /// Gyroscope data (in rad/s)
    pub gyroscope: Vector3,
    
    /// Battery level (0-100%)
    pub battery_level: u8,
    
    /// Temperature (in °C)
    pub temperature: f32,
}

/// Represents the state of the controller buttons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonState {
    /// Trigger button
    pub trigger: bool,
    
    /// Home button
    pub home: bool,
    
    /// Back button
    pub back: bool,
    
    /// Volume up button
    pub volume_up: bool,
    
    /// Volume down button
    pub volume_down: bool,
    
    /// Touchpad button (pressed)
    pub touchpad: bool,
}

/// Represents the state of the touchpad
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TouchpadState {
    /// Whether the touchpad is being touched
    pub touched: bool,
    
    /// X position on the touchpad (0.0 to 1.0, from left to right)
    pub x: f32,
    
    /// Y position on the touchpad (0.0 to 1.0, from bottom to top)
    pub y: f32,
}

/// Represents a quaternion for orientation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

/// Represents a 3D vector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Controller data parser
pub struct ControllerParser {
    /// Last received state
    last_state: Option<ControllerState>,
    
    /// Last update time
    last_update: Instant,
}

impl ControllerParser {
    /// Creates a new controller parser
    pub fn new() -> Self {
        Self {
            last_state: None,
            last_update: Instant::now(),
        }
    }
    
    /// Parses raw data from the controller
    pub fn parse_data(&mut self, data: &[u8]) -> Option<ControllerState> {
        if data.len() < 20 {
            return None; // Not enough data
        }
        
        // Parse button states
        let button_byte = data[0];
        let buttons = ButtonState {
            trigger: (button_byte & 0x01) != 0,
            home: (button_byte & 0x02) != 0,
            back: (button_byte & 0x04) != 0,
            touchpad: (button_byte & 0x08) != 0,
            volume_up: (button_byte & 0x10) != 0,
            volume_down: (button_byte & 0x20) != 0,
        };
        
        // Parse touchpad state
        let touchpad_byte = data[1];
        let touchpad_x = if data.len() > 2 { data[2] as f32 / 255.0 } else { 0.0 };
        let touchpad_y = if data.len() > 3 { data[3] as f32 / 255.0 } else { 0.0 };
        let touchpad = TouchpadState {
            touched: (touchpad_byte & 0x01) != 0,
            x: touchpad_x,
            y: touchpad_y,
        };
        
        // Parse sensor data (simplified - in a real implementation, this would be more complex)
        // For now, we'll just use placeholder values
        let orientation = Quaternion {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        };
        
        let accelerometer = Vector3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        
        let gyroscope = Vector3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        
        // Parse battery level and temperature (placeholder)
        let battery_level = 100; // 100%
        let temperature = 25.0; // 25°C
        
        // Create controller state
        let state = ControllerState {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_millis() as u64,
            buttons,
            touchpad,
            orientation,
            accelerometer,
            gyroscope,
            battery_level,
            temperature,
        };
        
        self.last_state = Some(state.clone());
        self.last_update = Instant::now();
        
        Some(state)
    }
    
    /// Gets the last known state
    pub fn get_last_state(&self) -> Option<ControllerState> {
        self.last_state.clone()
    }
    
    /// Gets the time since the last update
    pub fn time_since_last_update(&self) -> Duration {
        self.last_update.elapsed()
    }
}

impl Default for ControllerParser {
    fn default() -> Self {
        Self::new()
    }
}