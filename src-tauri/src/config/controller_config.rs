use crate::utils::ensure_directory_exists;
use anyhow::{Result};
use nalgebra::{Matrix3, Vector3};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tokio::fs;
use log::{error, info, warn};

const CONFIG_FILE_NAME: &str = "controller_config.json";
// 定义磁力计校准参数结构体
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MagCalibration {
    pub hard_iron_bias: Vector3<f64>,
    pub soft_iron_matrix: Matrix3<f64>,
}

// 定义陀螺仪校准参数结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GyroCalibration {
    pub zero_bias: Vector3<f64>,
}

impl Default for GyroCalibration {
    fn default() -> Self {
        Self {
            zero_bias: Vector3::zeros(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerConfig {
    /// 原始传感器数据 (加速度计、陀螺仪、磁力计) 低通滤波的 alpha 值。
    /// 控制传感器噪声的抑制程度。值越小滤波越强，延迟越大。
    pub sensor_low_pass_alpha: f64,

    /// 传感器时间步长 (delta_t) 平滑的 alpha 值。
    /// 控制时间步长的稳定性。值越小平滑越强，但可能引入更多延迟。
    pub delta_t_smoothing_alpha: f64,

    /// Madgwick 滤波器的 beta 参数。
    /// 控制对磁力计数据的信任程度。值越大，对磁力计的依赖越高，姿态收敛越快，但更容易受磁场干扰。
    pub madgwick_beta: f64,

    /// 地区地磁强度 (uT)
    pub local_earth_mag_field: f64,


    /// 磁力计校准参数
    pub mag_calibration: MagCalibration,

    /// 陀螺仪校准参数
    pub gyro_calibration: GyroCalibration,
}

impl Default for ControllerConfig {
    fn default() -> Self {
        let local_earth_mag_field = 49.5; // Shanghai ~49.5uT
        ControllerConfig {
            sensor_low_pass_alpha: 1.0,
            delta_t_smoothing_alpha: 1.0,
            madgwick_beta: 0.1,
            local_earth_mag_field,
            mag_calibration: MagCalibration::default(),
            gyro_calibration: GyroCalibration::default(),
        }
    }
}

impl ControllerConfig {
    /// Loads the config from a configuration file.
    pub async fn load_config(app_handle: &AppHandle) -> Result<Self> {
        let config_dir = app_handle.path().app_config_dir()?;
        let file_name = CONFIG_FILE_NAME.to_string();
        let file_path = config_dir.join(file_name);
        let file_path_str = file_path.to_string_lossy().into_owned();

        if !file_path.exists() {
            warn!(
                "Config file not found at {:?}, using default.",
                file_path_str
            );
            return Ok(Self::default());
        }

        let config_json = fs::read_to_string(file_path).await?;
        let config: Self = serde_json::from_str(&config_json)?;

        info!("Config loaded from {:?}", file_path_str);
        Ok(config)
    }

    /// Saves the current config to a configuration file.
    pub async fn save_config(&mut self, app_handle: &AppHandle) -> Result<()> {
        let config_dir = app_handle.path().app_config_dir()?;
        ensure_directory_exists(&config_dir).await?;

        let file_name = CONFIG_FILE_NAME.to_string();
        let file_path = config_dir.join(file_name);
        let file_path_str = file_path.to_string_lossy().into_owned();

        let config_json = match serde_json::to_string_pretty(&self) {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to serialize controller config to JSON: {}", e);
                return Err(e.into());
            }
        };

        fs::write(file_path.to_path_buf(), config_json).await?;

        info!("Controller config saved to {:?}.", file_path_str);
        Ok(())
    }
}

