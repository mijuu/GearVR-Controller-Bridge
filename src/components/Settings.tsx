import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// Assuming the config types from Rust are mirrored here.
// In a real scenario, these might be generated or shared types.
interface ControllerConfig {
  sensor_low_pass_alpha: number;
  delta_t_smoothing_alpha: number;
  local_earth_mag_field: number;
  mag_norm_max_threshold: number;
  mag_norm_min_threshold: number;
  // mag_calibration and gyro_calibration are handled separately
}

interface MouseMapperConfig {
  mode: 'AirMouse' | 'Touchpad';
  button_mapping: {
    trigger: string | null;
    home: string | null;
    back: string | null;
    volume_up: string | null;
    volume_down: string | null;
    touchpad: string | null;
  };
  air_mouse_sensitivity: number;
  touchpad_sensitivity: number;
  touchpad_acceleration: number;
  touchpad_acceleration_threshold: number;
  air_mouse_fov: number;
  air_mouse_activation_threshold: number;
}


interface SettingsProps {
  onBackToController: (view: 'controller') => void;
}

const Settings: React.FC<SettingsProps> = ({ onBackToController }) => {
  const [calibrationStatus, setCalibrationStatus] = useState('idle'); // idle, calibrating, success, failed
  const [calibrationStep, setCalibrationStep] = useState('');
  const [controllerConfig, setControllerConfig] = useState<ControllerConfig | null>(null);
  const [mouseMapperConfig, setMouseMapperConfig] = useState<MouseMapperConfig | null>(null);


  useEffect(() => {
    const unlistenStep = listen<string>('calibration-step', (event) => {
      setCalibrationStep(event.payload);
    });

    const unlistenFinished = listen<boolean>('calibration-finished', (event) => {
      setCalibrationStatus(event.payload ? 'success' : 'failed');
    });

    // Load initial configs
    invoke<ControllerConfig>('get_controller_config').then(setControllerConfig).catch(console.error);
    invoke<MouseMapperConfig>('get_mouse_mapper_config').then(setMouseMapperConfig).catch(console.error);


    return () => {
      unlistenStep.then(f => f());
      unlistenFinished.then(f => f());
    };
  }, []);

  const handleStartMagCalibration = async () => {
    try {
      setCalibrationStatus('calibrating');
      await invoke('start_mag_calibration_wizard');
    } catch (error) {
      console.error('Failed to start calibration:', error);
      setCalibrationStatus('failed');
    }
  };

  const handleStartGyroCalibration = async () => {
    try {
      // Assuming a similar wizard for gyro calibration
      alert("请将控制器静置在平坦表面上，然后点击“确定”开始陀螺仪校准。");
      await invoke('start_gyro_calibration');
      alert("陀螺仪校准完成！");
    } catch (error) {
      console.error('Failed to start gyro calibration:', error);
      alert("陀螺仪校准失败。");
    }
  };

  const handleSaveConfigs = async () => {
    try {
      if (controllerConfig) {
        await invoke('set_controller_config', { config: controllerConfig });
      }
      if (mouseMapperConfig) {
        await invoke('set_mouse_mapper_config', { config: mouseMapperConfig });
      }
      alert('配置已保存!');
    } catch (error) {
      console.error('Failed to save configs:', error);
      alert('配置保存失败!');
    }
  };

  const handleControllerChange = (field: keyof ControllerConfig, value: any) => {
    if (controllerConfig) {
      setControllerConfig({ ...controllerConfig, [field]: parseFloat(value) });
    }
  };

  const handleMouseMapperChange = (field: keyof MouseMapperConfig, value: any) => {
    if (mouseMapperConfig) {
        if (field === 'button_mapping') {
            setMouseMapperConfig({ ...mouseMapperConfig, button_mapping: value });
            return;
        }
      setMouseMapperConfig({ ...mouseMapperConfig, [field]: value });
    }
  };

  const handleButtonMappingChange = (button: keyof MouseMapperConfig['button_mapping'], value: string) => {
    if (mouseMapperConfig) {
        handleMouseMapperChange('button_mapping', {
            ...mouseMapperConfig.button_mapping,
            [button]: value || null,
        });
    }
  };


  return (
    <div style={styles.scrollContainer}>
    <div style={styles.container}>
      <h2 style={styles.heading}>设置</h2>

      {/* Calibration Section */}
      <div style={styles.section}>
        <h3 style={styles.subHeading}>校准</h3>
        <div style={styles.calibrationSection}>
            <h4 style={styles.subHeading4}>磁力计校准</h4>
            {calibrationStatus === 'idle' && (
              <button onClick={handleStartMagCalibration} style={styles.button}>
                开始校准
              </button>
            )}
            {calibrationStatus === 'calibrating' && (
              <div>
                <p>校准中... 请按以下步骤操作:</p>
                <p>{calibrationStep}</p>
              </div>
            )}
            {calibrationStatus === 'success' && (
              <div>
                <p>校准成功！</p>
                <button onClick={() => setCalibrationStatus('idle')} style={styles.button}>
                  再次校准
                </button>
              </div>
            )}
            {calibrationStatus === 'failed' && (
              <div>
                <p>校准失败，请重试。</p>
                <button onClick={handleStartMagCalibration} style={styles.button}>
                  重新校准
                </button>
              </div>
            )}
        </div>
        <div style={styles.calibrationSection}>
            <h4 style={styles.subHeading4}>陀螺仪校准</h4>
            <button onClick={handleStartGyroCalibration} style={styles.button}>
                开始校准
            </button>
        </div>
      </div>


      {/* Controller Settings */}
      {controllerConfig && (
        <div style={styles.section}>
          <h3 style={styles.subHeading}>控制器设置</h3>
          <div style={styles.formGroup}>
            <label>传感器低通滤波 (alpha)</label>
            <input type="number" step="0.01" value={controllerConfig.sensor_low_pass_alpha} onChange={(e) => handleControllerChange('sensor_low_pass_alpha', e.target.value)} style={styles.input} />
          </div>
          <div style={styles.formGroup}>
            <label>时间步长平滑 (alpha)</label>
            <input type="number" step="0.01" value={controllerConfig.delta_t_smoothing_alpha} onChange={(e) => handleControllerChange('delta_t_smoothing_alpha', e.target.value)} style={styles.input} />
          </div>
          <div style={styles.formGroup}>
            <label>本地地磁场强度 (μT)</label>
            <input type="number" step="0.1" value={controllerConfig.local_earth_mag_field} onChange={(e) => handleControllerChange('local_earth_mag_field', e.target.value)} style={styles.input} />
          </div>
          <div style={styles.formGroup}>
            <label>磁力计模长上限 (μT)</label>
            <input type="number" step="0.1" value={controllerConfig.mag_norm_max_threshold} onChange={(e) => handleControllerChange('mag_norm_max_threshold', e.target.value)} style={styles.input} />
          </div>
          <div style={styles.formGroup}>
            <label>磁力计模长下限 (μT)</label>
            <input type="number" step="0.1" value={controllerConfig.mag_norm_min_threshold} onChange={(e) => handleControllerChange('mag_norm_min_threshold', e.target.value)} style={styles.input} />
          </div>
        </div>
      )}

      {/* Mouse Mapper Settings */}
      {mouseMapperConfig && (
        <div style={styles.section}>
          <h3 style={styles.subHeading}>鼠标映射设置</h3>
          <div style={styles.formGroup}>
            <label>鼠标模式</label>
            <div style={styles.radioGroup}>
                <label>
                <input type="radio" value="AirMouse" checked={mouseMapperConfig.mode === 'AirMouse'} onChange={() => handleMouseMapperChange('mode', 'AirMouse')} />
                空中鼠标
                </label>
                <label>
                <input type="radio" value="Touchpad" checked={mouseMapperConfig.mode === 'Touchpad'} onChange={() => handleMouseMapperChange('mode', 'Touchpad')} />
                触摸板
                </label>
            </div>
          </div>

          <div style={styles.formGroup}>
            <label>空中鼠标灵敏度</label>
            <input type="number" step="0.1" value={mouseMapperConfig.air_mouse_sensitivity} onChange={(e) => handleMouseMapperChange('air_mouse_sensitivity', parseFloat(e.target.value))} style={styles.input} />
          </div>
          <div style={styles.formGroup}>
            <label>触摸板灵敏度</label>
            <input type="number" step="1" value={mouseMapperConfig.touchpad_sensitivity} onChange={(e) => handleMouseMapperChange('touchpad_sensitivity', parseFloat(e.target.value))} style={styles.input} />
          </div>
          <div style={styles.formGroup}>
            <label>触摸板加速度</label>
            <input type="number" step="0.1" value={mouseMapperConfig.touchpad_acceleration} onChange={(e) => handleMouseMapperChange('touchpad_acceleration', parseFloat(e.target.value))} style={styles.input} />
          </div>
          <div style={styles.formGroup}>
            <label>触摸板加速度阈值</label>
            <input type="number" step="0.0001" value={mouseMapperConfig.touchpad_acceleration_threshold} onChange={(e) => handleMouseMapperChange('touchpad_acceleration_threshold', parseFloat(e.target.value))} style={styles.input} />
          </div>
          <div style={styles.formGroup}>
            <label>空中鼠标FOV</label>
            <input type="number" step="1" value={mouseMapperConfig.air_mouse_fov} onChange={(e) => handleMouseMapperChange('air_mouse_fov', parseFloat(e.target.value))} style={styles.input} />
          </div>
          <div style={styles.formGroup}>
            <label>空中鼠标激活阈值</label>
            <input type="number" step="0.5" value={mouseMapperConfig.air_mouse_activation_threshold} onChange={(e) => handleMouseMapperChange('air_mouse_activation_threshold', parseFloat(e.target.value))} style={styles.input} />
          </div>

          <h4 style={styles.subHeading4}>按键映射</h4>
          {Object.entries(mouseMapperConfig.button_mapping).map(([key, value]) => (
            <div style={styles.formGroup} key={key}>
              <label>{key}</label>
              <input type="text" value={value ?? ''} onChange={(e) => handleButtonMappingChange(key as keyof MouseMapperConfig['button_mapping'], e.target.value)} style={styles.input} />
            </div>
          ))}
        </div>
      )}


      <div style={styles.footer}>
        <button onClick={handleSaveConfigs} style={styles.button}>
            保存设置
        </button>
        <button onClick={() => onBackToController('controller')} style={styles.backButton}>
            回到控制器界面
        </button>
      </div>
    </div>
    </div>
  );
};

const styles: { [key: string]: React.CSSProperties } = {
  scrollContainer: {
    height: '100%',
    overflowY: 'auto',
    backgroundColor: '#1e1e1e',
  },
  container: {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    // justifyContent: 'center', // Remove to allow scrolling
    padding: '20px',
    boxSizing: 'border-box',
    color: '#ffffff',
  },
  heading: {
    fontSize: '2.5rem',
    marginBottom: '30px',
    color: '#00ffcc',
  },
  section: {
    backgroundColor: '#2a2a2a',
    padding: '25px',
    borderRadius: '10px',
    marginBottom: '20px',
    width: '90%',
    maxWidth: '600px',
    boxShadow: '0 4px 8px rgba(0, 0, 0, 0.2)',
  },
  subHeading: {
    fontSize: '1.8rem',
    marginBottom: '20px',
    color: '#00ffcc',
    textAlign: 'center',
  },
  subHeading4: {
    fontSize: '1.2rem',
    marginTop: '15px',
    marginBottom: '10px',
    color: '#00ddb3',
    textAlign: 'center',
  },
  calibrationSection: {
    marginBottom: '15px',
    padding: '15px',
    border: '1px solid #444',
    borderRadius: '8px',
    textAlign: 'center',
  },
  formGroup: {
    marginBottom: '15px',
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'flex-start',
  },
  input: {
    width: '100%',
    padding: '8px',
    backgroundColor: '#333',
    border: '1px solid #555',
    borderRadius: '4px',
    color: '#fff',
    marginTop: '5px',
  },
  radioGroup: {
    display: 'flex',
    gap: '20px',
    marginTop: '5px',
  },
  button: {
    backgroundColor: '#00ffcc',
    color: '#1e1e1e',
    border: 'none',
    padding: '12px 25px',
    borderRadius: '5px',
    fontSize: '1.1rem',
    cursor: 'pointer',
    transition: 'background-color 0.3s ease',
    fontWeight: 'bold',
  },
  backButton: {
    backgroundColor: '#4a4a4a',
    color: '#ffffff',
    border: 'none',
    padding: '12px 25px',
    borderRadius: '5px',
    fontSize: '1.1rem',
    cursor: 'pointer',
    transition: 'background-color 0.3s ease',
    fontWeight: 'bold',
  },
  footer: {
    display: 'flex',
    justifyContent: 'space-around',
    width: '90%',
    maxWidth: '600px',
    marginTop: '30px',
  }
};

export default Settings;

