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
    
    /// Magnetometer data (in μT)
    pub magnetometer: Vector3,
    
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
        if data.len() < 59 {
            return None; // Not enough data
        }
        
        // Parse button states (from byte 58)
        let button_byte = data[58];
        let buttons = ButtonState {
            trigger: (button_byte & (1 << 0)) != 0,
            home: (button_byte & (1 << 1)) != 0,
            back: (button_byte & (1 << 2)) != 0,
            touchpad: (button_byte & (1 << 3)) != 0,
            volume_up: (button_byte & (1 << 4)) != 0,
            volume_down: (button_byte & (1 << 5)) != 0,
        };
        
        // Parse touchpad coordinates (from bytes 54-56)
        let axis_x = ((((data[54] as u16 & 0xF) << 6) + ((data[55] as u16 & 0xFC) >> 2)) & 0x3FFu16) as f32;
        let axis_y = ((((data[55] as u16 & 0x3) << 8) + (data[56] as u16 & 0xFF)) & 0x3FFu16) as f32;
        
        // Normalize to 0.0-1.0 range (max observed value is 315)
        let touchpad_x = (axis_x as f32 / 315.0).clamp(0.0, 1.0);
        let touchpad_y = (axis_y as f32 / 315.0).clamp(0.0, 1.0);
        
        let touchpad = TouchpadState {
            touched: touchpad_x > 0.0 || touchpad_y > 0.0,
            x: touchpad_x,
            y: touchpad_y,
        };
        
        // Parse accelerometer data (bytes 4-9)
        // Raw values are 16-bit signed integers (big-endian)
        // Conversion formula: raw * 9.80665 / 2048.0 (to m/s²)
        let accelerometer = Vector3 {
            x: (((data[4] as i16) << 8) | data[5] as i16) as f32 * 9.80665 / 2048.0,
            y: (((data[6] as i16) << 8) | data[7] as i16) as f32 * 9.80665 / 2048.0,
            z: (((data[8] as i16) << 8) | data[9] as i16) as f32 * 9.80665 / 2048.0,
        };
        
        // Parse gyroscope data (bytes 10-15) 
        // Raw values are 16-bit signed integers (big-endian)
        // Conversion formula: raw * (π/180) / 14.285 (to rad/s)
        let gyroscope = Vector3 {
            x: (((data[10] as i16) << 8) | data[11] as i16) as f32 * 0.017453292 / 14.285,
            y: (((data[12] as i16) << 8) | data[13] as i16) as f32 * 0.017453292 / 14.285,
            z: (((data[14] as i16) << 8) | data[15] as i16) as f32 * 0.017453292 / 14.285,
        };
        
        // Calculate initial orientation from accelerometer (pitch and roll)
        let norm = (accelerometer.x * accelerometer.x + 
                   accelerometer.y * accelerometer.y + 
                   accelerometer.z * accelerometer.z).sqrt();
        
        let (pitch, roll) = if norm > 0.0 {
            let pitch = (-accelerometer.x / norm).asin();
            let roll = (accelerometer.y / norm).asin();
            (pitch, roll)
        } else {
            (0.0, 0.0)
        };
        
        // Convert to quaternion
        let cy = (pitch * 0.5).cos();
        let sy = (pitch * 0.5).sin();
        let cr = (roll * 0.5).cos();
        let sr = (roll * 0.5).sin();
        
        let orientation = Quaternion {
            x: sr * cy,
            y: cr * sy,
            z: cr * cy,
            w: sr * sy,
        };
        
        // Parse magnetometer data (bytes 48-54)
        // Raw values are 16-bit signed integers (little-endian)
        // Conversion formula: raw * 0.06 (to μT)
        let magnetometer = Vector3 {
            x: ((data[51] as i16) << 8 | data[50] as i16) as f32 * 0.06,
            y: ((data[49] as i16) << 8 | data[48] as i16) as f32 * 0.06,
            z: ((data[53] as i16) << 8 | data[52] as i16) as f32 * 0.06,
        };

        // Parse temperature (from byte 57)
        let temperature = data[57] as f32;
        
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
            magnetometer,
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