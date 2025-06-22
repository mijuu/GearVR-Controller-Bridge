//! GearVR Controller data parsing and handling
//! This module handles parsing and processing of data received from the GearVR controller.

use serde::{Deserialize, Serialize};
use std::time::{Duration};

use ahrs::{Madgwick, Ahrs}; 
use nalgebra::{Vector3, UnitQuaternion};


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
    pub orientation: UnitQuaternion<f64>, 
    
    /// Accelerometer data (in m/s²)
    pub accelerometer: Vector3<f64>,
    
    /// Gyroscope data (in rad/s)
    pub gyroscope: Vector3<f64>, 
    /// Magnetometer data (in μT)
    pub magnetometer: Vector3<f64>,
    
    /// Temperature (in °C)
    pub temperature: f64,
}

/// Represents the state of the controller buttons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonState {
    pub trigger: bool,
    pub home: bool,
    pub back: bool,
    pub volume_up: bool,
    pub volume_down: bool,
    pub touchpad: bool,
    pub no_button: bool,
}

/// Represents the state of the touchpad
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TouchpadState {
    pub touched: bool,
    pub x: f32,
    pub y: f32,
}

/// Controller data parser
pub struct ControllerParser {
    /// Last received state
    last_state: Option<ControllerState>,
    
    /// Last update time, used for delta_t calculation
    last_sensor_time: Option<f64>, 
    
    /// AHRS filter instance
    ahrs_filter: Madgwick<f64>, 
    
    /// The last orientation reported by the AHRS filter
    last_ahrs_orientation: UnitQuaternion<f64>,

    /// The last zero orientation
    last_zero_quaternion: Option<UnitQuaternion<f64>>,
}

impl ControllerParser {
    /// Creates a new controller parser
    pub fn new() -> Self {
        // 1 / 68.96 ?
        let sample_period: f64 = 0.014499999999998181; 
        let beta: f64 = 0.1;

        let ahrs_filter = Madgwick::<f64>::new(sample_period, beta); 
        
        Self {
            last_state: None,
            last_sensor_time: None, 
            ahrs_filter,
            last_ahrs_orientation: UnitQuaternion::identity(),
            last_zero_quaternion: None,
        }
    }
    
    /// Parses raw data from the controller
    pub fn parse_data(&mut self, data: &[u8]) -> Option<ControllerState> {
        if data.len() < 59 {
            return None; 
        }

        let button_byte = data[58];
        let buttons = ButtonState {
            trigger: (button_byte & (1 << 0)) != 0,
            home: (button_byte & (1 << 1)) != 0,
            back: (button_byte & (1 << 2)) != 0,
            touchpad: (button_byte & (1 << 3)) != 0,
            volume_up: (button_byte & (1 << 4)) != 0,
            volume_down: (button_byte & (1 << 5)) != 0,
            no_button: (button_byte & (1 << 6)) != 0,
        };
        
        let axis_x = ((((data[54] as u16 & 0xF) << 6) + ((data[55] as u16 & 0xFC) >> 2)) & 0x3FFu16) as f64;
        let axis_y = ((((data[55] as u16 & 0x3) << 8) + (data[56] as u16 & 0xFF)) & 0x3FFu16) as f64;
        
        let touchpad_x = (axis_x as f32 / 315.0).clamp(0.0, 1.0);
        let touchpad_y = (axis_y as f32 / 315.0).clamp(0.0, 1.0);
        
        let touchpad = TouchpadState {
            touched: touchpad_x > 0.0 || touchpad_y > 0.0,
            x: touchpad_x,
            y: touchpad_y,
        };

        // 9.80665 / 2048.0 = 0.00478840332
        let acc_val_factor = 0.00478840332;
        let raw_accel = Vector3::new(
            i16::from_le_bytes([data[4], data[5]]) as f64 * acc_val_factor,
            i16::from_le_bytes([data[6], data[7]]) as f64 * acc_val_factor,
            i16::from_le_bytes([data[8], data[9]]) as f64 * acc_val_factor,
        );
        let accelerometer = Vector3::new(raw_accel.x, raw_accel.y, raw_accel.z);

        // 0.017453292 / 14.285 = 0.001221791529
        let gyr_val_factor = 0.001221791529;
        let raw_gyro = Vector3::new(
            i16::from_le_bytes([data[10], data[11]]) as f64 * gyr_val_factor, 
            i16::from_le_bytes([data[12], data[13]]) as f64 * gyr_val_factor,
            i16::from_le_bytes([data[14], data[15]]) as f64 * gyr_val_factor,
        );
        let gyroscope = Vector3::new(raw_gyro.x, raw_gyro.y, raw_gyro.z);

        let mag_val_factor = 0.06;
        let raw_mag = Vector3::new(
            i16::from_le_bytes([data[48], data[49]]) as f64 * mag_val_factor,
            i16::from_le_bytes([data[50], data[51]]) as f64 * mag_val_factor,
            i16::from_le_bytes([data[52], data[53]]) as f64 * mag_val_factor,
        );
        let magnetometer = Vector3::new(raw_mag.x, raw_mag.y, raw_mag.z);
        
        let temperature = data[57] as f64;

        // --- AHRS 集成部分 ---
        // 时间是data的0-3字节, 默认是微秒
        let current_sensor_time_seconds = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as f64 / 1_000_000.0;

        // --- 计算 delta_t ---
        let mut delta_t: f64;
        if let Some(prev_time) = self.last_sensor_time {
            delta_t = current_sensor_time_seconds - prev_time;

            const MIN_ACCEPTABLE_DELTA_T: f64 = 1.0 / 2000.0; // 例如，假定最高频率是 2000Hz
            const MAX_ACCEPTABLE_DELTA_T: f64 = 1.0 / 10.0;  // 例如，假定最低频率是 10Hz
            delta_t = delta_t.max(MIN_ACCEPTABLE_DELTA_T).min(MAX_ACCEPTABLE_DELTA_T);
            // 处理首次连接或时间戳回绕（如果发生的话）
            if delta_t <= 0.0 { 
                // 时间戳没有前进，或者发生了回绕，这会导致 AHRS 异常
                // 打印警告或使用一个默认的 delta_t，例如 initial_sample_period
                eprintln!("Warning: Non-positive delta_t: {}. Using default sample_period.", delta_t);
            }
        } else {
            // 第一次解析数据，无法计算 delta_t。
            // 使用 Madgwick 构造时提供的 initial_sample_period 作为首次 delta_t
            delta_t = self.ahrs_filter.sample_period();
            eprintln!("First sensor data, using initial_sample_period as delta_t: {}", delta_t);
        }
        self.last_sensor_time = Some(current_sensor_time_seconds); 

        // 归一化加速度计数据 
        let nalgebra_accel = accelerometer.normalize();

        // 使用ahrs feature field_access
        let sample_period_ref: &mut f64 = self.ahrs_filter.sample_period_mut();
        *sample_period_ref = delta_t;

        // 更新 AHRS 滤波器。
        let update_result = self.ahrs_filter.update(&gyroscope, &nalgebra_accel, &magnetometer);
        // 如果更新失败，打印错误并返回 None（或保留上次的姿态）
        if let Err(e) = update_result {
            eprintln!("AHRS update failed: {:?}", e); 
            // 为了平滑过渡，如果AHRS更新失败，我们使用上一次成功的姿态
            let orientation = self.last_ahrs_orientation;

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
            return Some(state);
        }

        // 如果更新成功，获取新的姿态
        let orientation = self.ahrs_filter.quat; 
        self.last_ahrs_orientation = orientation;

        let mut final_display_orientation = orientation;
        if buttons.home {
            // 记录当前的 AHRS 四元数的逆
            self.last_zero_quaternion = Some(orientation.inverse()); // 记录未经过归零的 AHRS 输出的逆
            eprintln!("Re-zeroed orientation!");
        }
        if let Some(zero_q) = self.last_zero_quaternion {
            // 应用归零转换
            final_display_orientation = zero_q * final_display_orientation;
        }

        let state = ControllerState {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_millis() as u64,
            buttons,
            touchpad,
            orientation: final_display_orientation,
            accelerometer,
            gyroscope,
            magnetometer,
            temperature,
        };
        
        self.last_state = Some(state.clone());
        
        Some(state)
    }
    
}

impl Default for ControllerParser {
    fn default() -> Self {
        Self::new()
    }
}