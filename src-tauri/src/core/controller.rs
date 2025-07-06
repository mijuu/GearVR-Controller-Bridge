//! GearVR Controller data parsing and handling
//! This module handles parsing and processing of data received from the GearVR controller.

use serde::{Deserialize, Serialize};
use std::time::{Duration};
use std::path::Path;

use ahrs::{Madgwick, Ahrs}; 
use nalgebra::{Vector3, UnitQuaternion, Matrix3};
use std::sync::{Arc};
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use anyhow::{Result, anyhow};

use crate::config::controller_config::{ControllerConfig};

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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

    /// smoothed delta_t
    smoothed_delta_t: f64,

    /// filtered sensor data
    last_filtered_accel: Vector3<f64>,
    last_filtered_gyro: Vector3<f64>,
    last_filtered_mag: Vector3<f64>,

    pub config: ControllerConfig,

    /// Sender for data recording
    data_record_sender: Option<mpsc::Sender<String>>,

    /// Recorded magnetometer data for calibration
    recorded_mag_data: Arc<Mutex<Vec<Vector3<f64>>>>,
    /// Recorded gyroscope data for calibration
    recorded_gyro_data: Arc<Mutex<Vec<Vector3<f64>>>>,
}

impl ControllerParser {
    /// Creates a new controller parser
    pub fn new(config: ControllerConfig) -> Self {
        // 1 / 68.96 ?
        let sample_period: f64 = 0.014499999999998181; 
        let beta = config.madgwick_beta;

        let ahrs_filter = Madgwick::<f64>::new(sample_period, beta); 
        
        Self {
            last_state: None,
            last_sensor_time: None, 
            ahrs_filter,
            last_ahrs_orientation: UnitQuaternion::identity(),
            last_zero_quaternion: None,
            smoothed_delta_t: sample_period,
            last_filtered_accel: Vector3::zeros(),
            last_filtered_gyro: Vector3::zeros(),
            last_filtered_mag: Vector3::zeros(),
            config,
            data_record_sender: None,
            recorded_mag_data: Arc::new(Mutex::new(Vec::new())),
            recorded_gyro_data: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Updates the configuration of the controller parser and re-initializes components.
    pub fn update_config(&mut self, new_config: ControllerConfig) {
        // Re-initialize the AHRS filter with the new beta value
        let sample_period = self.ahrs_filter.sample_period(); // Keep the last known sample period
        self.ahrs_filter = Madgwick::<f64>::new(sample_period, new_config.madgwick_beta);
        
        // Update the config struct itself
        self.config = new_config;
        
        log::info!("ControllerParser config updated. New beta: {}", self.config.madgwick_beta);
    }

    /// Starts recording sensor data to a CSV file.
    pub fn start_data_recording(&mut self, file_path: &Path) {
        let (tx, mut rx) = mpsc::channel::<String>(100); // Buffer for 100 messages
        self.data_record_sender = Some(tx);

        let recorded_mag_data_arc = self.recorded_mag_data.clone(); // Clone for the async task
        let recorded_gyro_data_arc = self.recorded_gyro_data.clone();
        let file_path_str = file_path.to_string_lossy().into_owned(); // Clone for the async task
        let task_file_path_str = file_path_str.clone();
        let task_path = file_path.to_path_buf();

        tokio::spawn(async move {
            let mut file = match File::create(&task_path).await {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Failed to create data recording file {}: {}", task_file_path_str, e);
                    return;
                }
            };

            // Write CSV header
            if let Err(e) = file.write_all(b"timestamp_us,accel_x,accel_y,accel_z,gyro_x,gyro_y,gyro_z,mag_x,mag_y,mag_z\n").await {
                eprintln!("Failed to write CSV header to {}: {}", task_file_path_str, e);
                return;
            }

            let mut recorded_mag_data_guard = recorded_mag_data_arc.lock().await;
            recorded_mag_data_guard.clear(); // Clear previous data

            let mut recorded_gyro_data_guard = recorded_gyro_data_arc.lock().await;
            recorded_gyro_data_guard.clear(); // Clear previous data

            while let Some(data_line) = rx.recv().await {
                // Parse mag and gyro data from line and push to respective recorded_data_guard
                let parts: Vec<&str> = data_line.trim().split(',').collect();
                if parts.len() == 10 {
                    if let (Ok(_accel_x), Ok(_accel_y), Ok(_accel_z),
                            Ok(gyro_x), Ok(gyro_y), Ok(gyro_z),
                            Ok(mag_x), Ok(mag_y), Ok(mag_z)) = (
                        parts[1].parse::<f64>(), parts[2].parse::<f64>(), parts[3].parse::<f64>(),
                        parts[4].parse::<f64>(), parts[5].parse::<f64>(), parts[6].parse::<f64>(),
                        parts[7].parse::<f64>(), parts[8].parse::<f64>(), parts[9].parse::<f64>(),
                    ) {
                        recorded_mag_data_guard.push(Vector3::new(mag_x, mag_y, mag_z));
                        recorded_gyro_data_guard.push(Vector3::new(gyro_x, gyro_y, gyro_z));
                    }
                }

                if let Err(e) = file.write_all(data_line.as_bytes()).await {
                    eprintln!("Failed to write data to {}: {}", task_file_path_str, e);
                    break;
                }
            }
            eprintln!("Data recording to {} stopped.", task_file_path_str);
        });
        eprintln!("Data recording to {} started.", file_path_str);
    }

    /// Stops recording sensor data.
    pub fn stop_data_recording(&mut self) {
        self.data_record_sender.take(); // Drop the sender, which will close the channel
        eprintln!("Stopping data recording.");
    }
    
    /// Clears recorded sensor data.
    /// If `clear_mag` is true, clears magnetometer data.
    /// If `clear_gyro` is true, clears gyroscope data.
    pub async fn clear_recorded_data(&mut self, clear_mag: bool, clear_gyro: bool) {
        if clear_mag {
            let mut mag_data = self.recorded_mag_data.lock().await;
            mag_data.clear();
            eprintln!("Cleared recorded magnetometer data.");
        }
        if clear_gyro {
            let mut gyro_data = self.recorded_gyro_data.lock().await;
            gyro_data.clear();
            eprintln!("Cleared recorded gyroscope data.");
        }
    }
    
    /// Performs magnetometer calibration using recorded data.
    pub async fn perform_mag_calibration(&mut self) -> Result<()> {
        let recorded_mag_data_guard = self.recorded_mag_data.lock().await;
        let mag_data = &*recorded_mag_data_guard;

        if mag_data.is_empty() {
            return Err(anyhow!("No magnetometer data recorded for calibration."));
        }

        // --- 简化的椭球拟合算法占位符 ---
        // 实际的椭球拟合算法会更复杂，通常需要外部库或更详细的数学实现。
        // 这里我们只是计算一个简单的平均值作为硬铁偏置的估计，
        // 软铁矩阵暂时设为单位矩阵。
        // 这是一个非常简化的示例，仅用于演示流程。
        // 真正的校准需要确保数据覆盖所有方向，并使用最小二乘法等方法拟合椭球。

        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_z = 0.0;
        for v in mag_data.iter() {
            sum_x += v.x;
            sum_y += v.y;
            sum_z += v.z;
        }

        let count = mag_data.len() as f64;
        let estimated_hard_iron_bias = Vector3::new(sum_x / count, sum_y / count, sum_z / count);
        let estimated_soft_iron_matrix = Matrix3::identity(); // 暂时使用单位矩阵

        self.config.mag_calibration.hard_iron_bias = estimated_hard_iron_bias;
        self.config.mag_calibration.soft_iron_matrix = estimated_soft_iron_matrix;

        eprintln!("Magnetometer calibration performed.");
        eprintln!("Estimated Hard Iron Bias: {:?}", self.config.mag_calibration.hard_iron_bias);
        eprintln!("Estimated Soft Iron Matrix: {:?}", self.config.mag_calibration.soft_iron_matrix);

        Ok(())
    }

    /// Performs gyroscope calibration using recorded data.
    pub async fn perform_gyro_calibration(&mut self) -> Result<()> {
        let recorded_gyro_data_guard = self.recorded_gyro_data.lock().await;
        let gyro_data = &*recorded_gyro_data_guard;

        if gyro_data.is_empty() {
            return Err(anyhow!("No gyroscope data recorded for calibration."));
        }

        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_z = 0.0;
        for v in gyro_data.iter() {
            sum_x += v.x;
            sum_y += v.y;
            sum_z += v.z;
        }

        let count = gyro_data.len() as f64;
        let estimated_gyro_bias = Vector3::new(sum_x / count, sum_y / count, sum_z / count);

        self.config.gyro_calibration.zero_bias = estimated_gyro_bias;

        eprintln!("Gyroscope calibration performed.");
        eprintln!("Estimated Gyro Bias: {:?}", self.config.gyro_calibration.zero_bias);

        Ok(())
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
        let raw_accelerometer = Vector3::new(
            i16::from_le_bytes([data[4], data[5]]) as f64 * acc_val_factor,
            i16::from_le_bytes([data[6], data[7]]) as f64 * acc_val_factor,
            i16::from_le_bytes([data[8], data[9]]) as f64 * acc_val_factor,
        );

        // 0.017453292 / 14.285 = 0.001221791529
        let gyr_val_factor = 0.001221791529;
        let raw_gyroscope = Vector3::new(
            i16::from_le_bytes([data[10], data[11]]) as f64 * gyr_val_factor,
            i16::from_le_bytes([data[12], data[13]]) as f64 * gyr_val_factor,
            i16::from_le_bytes([data[14], data[15]]) as f64 * gyr_val_factor,
        );

        // Y, X, Z
        let mag_val_factor = 0.0045;
        let raw_magnetometer = Vector3::new(
            i16::from_le_bytes([data[48], data[49]]) as f64 * mag_val_factor,
            i16::from_le_bytes([data[50], data[51]]) as f64 * mag_val_factor,
            i16::from_le_bytes([data[52], data[53]]) as f64 * mag_val_factor,
        );

        // Record raw data if recording is active
        if let Some(sender) = &self.data_record_sender {
            let timestamp_us = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as u64;
            let line = format!("{},{},{},{},{},{},{},{},{},{}\n",
                timestamp_us,
                raw_accelerometer.x, raw_accelerometer.y, raw_accelerometer.z,
                raw_gyroscope.x, raw_gyroscope.y, raw_gyroscope.z,
                raw_magnetometer.x, raw_magnetometer.y, raw_magnetometer.z
            );
            if let Err(e) = sender.try_send(line) {
                eprintln!("Failed to send data for recording: {}", e);
            }
        }

        // Apply calibration for real-time use and AHRS
        let calibrated_gyro = raw_gyroscope - self.config.gyro_calibration.zero_bias;
        let calibrated_mag = self.config.mag_calibration.soft_iron_matrix * (raw_magnetometer - self.config.mag_calibration.hard_iron_bias);

        let filter_alpha_sensor = self.config.sensor_low_pass_alpha;
        let current_accel_filtered = raw_accelerometer * filter_alpha_sensor + self.last_filtered_accel * (1.0 - filter_alpha_sensor);
        let current_gyro_filtered = calibrated_gyro * filter_alpha_sensor + self.last_filtered_gyro * (1.0 - filter_alpha_sensor);
        let current_mag_filtered = calibrated_mag * filter_alpha_sensor + self.last_filtered_mag * (1.0 - filter_alpha_sensor);

        self.last_filtered_accel = current_accel_filtered;
        self.last_filtered_gyro = current_gyro_filtered;
        self.last_filtered_mag = current_mag_filtered;
        
        let temperature = data[57] as f64;

        // --- AHRS 集成部分 ---
        // 时间是data的0-3字节, 默认是微秒
        let current_sensor_time_seconds = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as f64 / 1_000_000.0;

        // --- 计算 delta_t ---
        let delta_t: f64;
        if let Some(prev_time) = self.last_sensor_time {
            delta_t = current_sensor_time_seconds - prev_time;

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
        let nalgebra_accel = current_accel_filtered.normalize();

        // smoothed_delta_t 用于计算 AHRS 的 sample_period，以平滑过渡。
        let alpha = self.config.delta_t_smoothing_alpha;
        self.smoothed_delta_t = alpha * delta_t + (1.0 - alpha) * self.smoothed_delta_t;
        // 使用ahrs feature field_access
        let sample_period_ref: &mut f64 = self.ahrs_filter.sample_period_mut();
        *sample_period_ref = self.smoothed_delta_t;

        // 检查磁力计数据是否在有效范围内
        let mag_norm_max_threshold = self.config.local_earth_mag_field * 1.2; // 20% margin
        let mag_norm_min_threshold = self.config.local_earth_mag_field * 0.8; // 20% margin

        let mag_norm = calibrated_mag.norm(); // 使用校准后的磁力计数据进行范数检查
        let update_result = if mag_norm > mag_norm_min_threshold && mag_norm < mag_norm_max_threshold {
            // eprintln!("Magnetometer data in range (norm: {}), using full AHRS update.", mag_norm);
            self.ahrs_filter.update(&current_gyro_filtered, &nalgebra_accel, &calibrated_mag) // 使用校准后的磁力计数据
        } else {
            // 磁力计数据无效（可能受到干扰），仅使用 IMU 更新
            // eprintln!("Magnetometer data out of range (norm: {}), using IMU update only.", mag_norm);
            self.ahrs_filter.update_imu(&current_gyro_filtered, &nalgebra_accel)
        };
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
                accelerometer: current_accel_filtered,
                gyroscope: current_gyro_filtered,
                magnetometer: current_mag_filtered,
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
            accelerometer: current_accel_filtered,
            gyroscope: current_gyro_filtered,
            magnetometer: current_mag_filtered,
            temperature,
        };
        
        self.last_state = Some(state.clone());
        
        Some(state)
    }
    
}

impl Default for ControllerParser {
    fn default() -> Self {
        Self::new(ControllerConfig::default())
    }
}