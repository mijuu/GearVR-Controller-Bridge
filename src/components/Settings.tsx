import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// --- Type Definitions to match Rust structs ---
type Vector3 = [number, number, number];
type Matrix3 = [number, number, number, number, number, number, number, number, number];

interface MagCalibration {
    hard_iron_bias: Vector3;
    soft_iron_matrix: Matrix3;
}

interface GyroCalibration {
    zero_bias: Vector3;
}

interface ControllerConfig {
  sensor_low_pass_alpha: number;
  delta_t_smoothing_alpha: number;
  local_earth_mag_field: number;
  mag_calibration: MagCalibration;
  gyro_calibration: GyroCalibration;
}

interface MouseMapperConfig {
  mode: 'AirMouse' | 'Touchpad';
  button_mapping: { [key: string]: string | null };
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

type ActiveMenu = 'calibration' | 'controller' | 'mouse';
type CalibrationStatus = 'idle' | 'calibrating' | 'success' | 'failed';
type ToastType = 'success' | 'error';

// --- Sub-components ---

const CalibrationCard: React.FC<any> = ({ title, description, status, calibrationStep, onStart }) => {
    const renderStatus = () => {
        switch (status) {
            case 'calibrating': return <div style={styles.statusIndicatorCalibrating}>校准中...</div>;
            case 'success': return <div style={styles.statusIndicatorSuccess}>✓ 校准成功</div>;
            case 'failed': return <div style={styles.statusIndicatorFailed}>✗ 校准失败</div>;
            default: return null;
        }
    };

    return (
        <div style={styles.card}>
            <div style={styles.cardHeader}>
                <h4 style={styles.cardTitle}>{title}</h4>
                {renderStatus()}
            </div>
            <div style={styles.cardBody}>
                {status === 'calibrating' ? (
                    <div style={styles.calibrationProgress}>
                        <p>{calibrationStep || '请按指示操作...'}</p>
                    </div>
                ) : (
                    <p style={styles.cardDescription}>{description}</p>
                )}
            </div>
            <div style={styles.cardFooter}>
                <button onClick={status === 'calibrating' ? undefined : onStart} disabled={status === 'calibrating'} style={styles.button}>
                    {status === 'success' || status === 'failed' ? '重新校准' : '开始校准'}
                </button>
            </div>
        </div>
    );
};

const MatrixDisplay: React.FC<{ matrix: Matrix3 }> = ({ matrix }) => (
    <div style={styles.matrixContainer}>
        {matrix.map((val, index) => (
            <div key={index} style={styles.matrixCell}>{val.toFixed(4)}</div>
        ))}
    </div>
);

const VectorDisplay: React.FC<{ vector: Vector3, labels?: [string, string, string] }> = ({ vector, labels = ['X', 'Y', 'Z'] }) => (
    <div style={styles.vectorContainer}>
        {vector.map((val, index) => (
            <div key={index} style={styles.vectorItem}>
                <span style={styles.vectorLabel}>{labels[index]}:</span>
                <span>{val.toFixed(4)}</span>
            </div>
        ))}
    </div>
);

// --- Main Settings Component ---
const Settings: React.FC<SettingsProps> = ({ onBackToController }) => {
  const [magCalibrationStatus, setMagCalibrationStatus] = useState<CalibrationStatus>('idle');
  const [gyroCalibrationStatus, setGyroCalibrationStatus] = useState<CalibrationStatus>('idle');
  const [calibrationStep, setCalibrationStep] = useState('');
  const [controllerConfig, setControllerConfig] = useState<ControllerConfig | null>(null);
  const [mouseMapperConfig, setMouseMapperConfig] = useState<MouseMapperConfig | null>(null);
  const [activeMenu, setActiveMenu] = useState<ActiveMenu>('calibration');
  const [toast, setToast] = useState<{ message: string; type: ToastType } | null>(null);

  const showToast = (message: string, type: ToastType = 'success') => {
    setToast({ message, type });
    setTimeout(() => setToast(null), 3000);
  };

  useEffect(() => {
    const unlistenStep = listen<string>('calibration-step', (event) => setCalibrationStep(event.payload));
    const unlistenFinished = listen<boolean>('calibration-finished', (event) => {
      if (magCalibrationStatus === 'calibrating') {
        setMagCalibrationStatus(event.payload ? 'success' : 'failed');
        // Refresh config to show new calibration data
        invoke<ControllerConfig>('get_controller_config').then(setControllerConfig);
      }
    });

    // Initial load
    if (controllerConfig === null) {
        invoke<ControllerConfig>('get_controller_config').then(setControllerConfig).catch(console.error);
    }
    if (mouseMapperConfig === null) {
        invoke<MouseMapperConfig>('get_mouse_mapper_config').then(setMouseMapperConfig).catch(console.error);
    }

    return () => {
      unlistenStep.then(f => f());
      unlistenFinished.then(f => f());
    };
  }, [magCalibrationStatus]); // Dependency ensures the listener always has the latest status

  const handleStartMagCalibration = async () => {
    try {
      setMagCalibrationStatus('calibrating');
      setCalibrationStep('请拿起控制器，在空中画8字形...');
      await invoke('start_mag_calibration_wizard');
    } catch (error) {
      console.error('Failed to start mag calibration:', error);
      setMagCalibrationStatus('failed');
    }
  };

  const handleStartGyroCalibration = async () => {
    setGyroCalibrationStatus('calibrating');
    setCalibrationStep('请将控制器静置在平坦表面上...');
    try {
      await invoke('start_gyro_calibration');
      setGyroCalibrationStatus('success');
      invoke<ControllerConfig>('get_controller_config').then(setControllerConfig);
    } catch (error) {
      console.error('Failed to start gyro calibration:', error);
      setGyroCalibrationStatus('failed');
    }
  };

  const handleConfigChange = (type: 'controller' | 'mouse', field: string, value: any, isButtonMapping = false) => {
    let newConfig;
    if (type === 'controller') {
        if (!controllerConfig) return;
        newConfig = { ...controllerConfig, [field]: parseFloat(value) };
        setControllerConfig(newConfig);
    } else {
        if (!mouseMapperConfig) return;
        if (isButtonMapping) {
            const newButtonMapping = { ...mouseMapperConfig.button_mapping, [field]: value || null };
            newConfig = { ...mouseMapperConfig, button_mapping: newButtonMapping };
        } else {
            newConfig = { ...mouseMapperConfig, [field]: value };
        }
        setMouseMapperConfig(newConfig);
    }

    const command = type === 'controller' ? 'set_controller_config' : 'set_mouse_mapper_config';
    invoke(command, { config: newConfig })
        .then(() => showToast(`${type === 'controller' ? '控制器' : '鼠标'}设置已保存`, 'success'))
        .catch(err => {
            showToast('保存失败', 'error');
            console.error(`Failed to save ${type} config:`, err);
        });
  };

  const renderContent = () => {
    switch (activeMenu) {
      case 'calibration':
        return (
          <div style={styles.section}>
            <h3 style={styles.subHeading}>传感器校准</h3>
            <div style={styles.cardsContainer}>
                <CalibrationCard title="磁力计校准" description="用于修正方向漂移，提高指向精度。" status={magCalibrationStatus} calibrationStep={magCalibrationStatus === 'calibrating' ? calibrationStep : undefined} onStart={handleStartMagCalibration} />
                <CalibrationCard title="陀螺仪校准" description="用于修正旋转过程中的抖动和偏移。" status={gyroCalibrationStatus} calibrationStep={gyroCalibrationStatus === 'calibrating' ? '校准中，请保持设备静止...' : undefined} onStart={handleStartGyroCalibration} />
            </div>
          </div>
        );
      case 'controller':
        const editableControllerFields = [
            { key: 'sensor_low_pass_alpha', label: '传感器低通滤波 (alpha)' },
            { key: 'delta_t_smoothing_alpha', label: '时间步长平滑 (alpha)' },
            { key: 'local_earth_mag_field', label: '本地地磁场强度 (μT)' },
        ];
        return controllerConfig && (
            <div style={styles.section}>
              <h3 style={styles.subHeading}>控制器设置</h3>
              {editableControllerFields.map(({ key, label }) => (
                <div style={styles.formGroup} key={key}>
                  <label>{label}</label>
                  <input type="number" step="0.01" value={(controllerConfig as any)[key]} onBlur={(e) => handleConfigChange('controller', key, e.target.value)} onChange={(e) => setControllerConfig({ ...controllerConfig, [key]: parseFloat(e.target.value) || 0 })} style={styles.input} />
                </div>
              ))}
              <h4 style={styles.subHeading4}>陀螺仪校准数据 (只读)</h4>
              <VectorDisplay vector={controllerConfig.gyro_calibration.zero_bias} />

              <h4 style={styles.subHeading4}>磁力计校准数据 (只读)</h4>
              <label>Hard Iron Bias</label>
              <VectorDisplay vector={controllerConfig.mag_calibration.hard_iron_bias} />
              <label style={{marginTop: '10px'}}>Soft Iron Matrix</label>
              <MatrixDisplay matrix={controllerConfig.mag_calibration.soft_iron_matrix} />
            </div>
          );
      case 'mouse':
        return mouseMapperConfig && (
            <div style={styles.section}>
              <h3 style={styles.subHeading}>鼠标映射设置</h3>
              <div style={styles.formGroup}>
                <label>鼠标模式</label>
                <div style={styles.radioGroup}>
                    <label>
                    <input type="radio" value="AirMouse" checked={mouseMapperConfig.mode === 'AirMouse'} onChange={() => handleConfigChange('mouse', 'mode', 'AirMouse')} />
                    空中鼠标
                    </label>
                    <label>
                    <input type="radio" value="Touchpad" checked={mouseMapperConfig.mode === 'Touchpad'} onChange={() => handleConfigChange('mouse', 'mode', 'Touchpad')} />
                    触摸板
                    </label>
                </div>
              </div>
    
              <div style={styles.formGroup}>
                <label>空中鼠标灵敏度</label>
                <input type="number" step="0.1" value={mouseMapperConfig.air_mouse_sensitivity} onBlur={(e) => handleConfigChange('mouse', 'air_mouse_sensitivity', parseFloat(e.target.value))} onChange={(e) => setMouseMapperConfig({...mouseMapperConfig, air_mouse_sensitivity: parseFloat(e.target.value)})} style={styles.input} />
              </div>
              <div style={styles.formGroup}>
                <label>触摸板灵敏度</label>
                <input type="number" step="1" value={mouseMapperConfig.touchpad_sensitivity} onBlur={(e) => handleConfigChange('mouse', 'touchpad_sensitivity', parseFloat(e.target.value))} onChange={(e) => setMouseMapperConfig({...mouseMapperConfig, touchpad_sensitivity: parseFloat(e.target.value)})} style={styles.input} />
              </div>
              <div style={styles.formGroup}>
                <label>触摸板加速度</label>
                <input type="number" step="0.1" value={mouseMapperConfig.touchpad_acceleration} onBlur={(e) => handleConfigChange('mouse', 'touchpad_acceleration', parseFloat(e.target.value))} onChange={(e) => setMouseMapperConfig({...mouseMapperConfig, touchpad_acceleration: parseFloat(e.target.value)})} style={styles.input} />
              </div>
              <div style={styles.formGroup}>
                <label>触摸板加速度阈值</label>
                <input type="number" step="0.0001" value={mouseMapperConfig.touchpad_acceleration_threshold} onBlur={(e) => handleConfigChange('mouse', 'touchpad_acceleration_threshold', parseFloat(e.target.value))} onChange={(e) => setMouseMapperConfig({...mouseMapperConfig, touchpad_acceleration_threshold: parseFloat(e.target.value)})} style={styles.input} />
              </div>
              <div style={styles.formGroup}>
                <label>空中鼠标FOV</label>
                <input type="number" step="1" value={mouseMapperConfig.air_mouse_fov} onBlur={(e) => handleConfigChange('mouse', 'air_mouse_fov', parseFloat(e.target.value))} onChange={(e) => setMouseMapperConfig({...mouseMapperConfig, air_mouse_fov: parseFloat(e.target.value)})} style={styles.input} />
              </div>
              <div style={styles.formGroup}>
                <label>空中鼠标激活阈值</label>
                <input type="number" step="0.5" value={mouseMapperConfig.air_mouse_activation_threshold} onBlur={(e) => handleConfigChange('mouse', 'air_mouse_activation_threshold', parseFloat(e.target.value))} onChange={(e) => setMouseMapperConfig({...mouseMapperConfig, air_mouse_activation_threshold: parseFloat(e.target.value)})} style={styles.input} />
              </div>
    
              <h4 style={styles.subHeading4}>按键映射</h4>
              {Object.entries(mouseMapperConfig.button_mapping).map(([key, value]) => (
                <div style={styles.formGroup} key={key}>
                  <label>{key}</label>
                  <input type="text" value={value ?? ''} onBlur={(e) => handleConfigChange('mouse', key, e.target.value, true)} onChange={(e) => setMouseMapperConfig({...mouseMapperConfig, button_mapping: {...mouseMapperConfig.button_mapping, [key]: e.target.value || null}})} style={styles.input} />
                </div>
              ))}
            </div>
          );
      default:
        return null;
    }
  }

  return (
    <div style={styles.page}>
        <div style={styles.container}>
            <div style={styles.leftMenu}>
                <h2 style={styles.heading}>设置</h2>
                <button style={activeMenu === 'calibration' ? styles.menuButtonActive : styles.menuButton} onClick={() => setActiveMenu('calibration')}>传感器校准</button>
                <button style={activeMenu === 'controller' ? styles.menuButtonActive : styles.menuButton} onClick={() => setActiveMenu('controller')}>控制器设置</button>
                <button style={activeMenu === 'mouse' ? styles.menuButtonActive : styles.menuButton} onClick={() => setActiveMenu('mouse')}>鼠标映射设置</button>
                <button style={{...styles.menuButton, marginTop: 'auto'}} onClick={() => onBackToController('controller')}>← 返回控制器界面</button>
            </div>
            <div style={styles.rightContent}>
                {renderContent()}
            </div>
        </div>
        {toast && <div style={{...styles.toastBase, ...(toast.type === 'success' ? styles.toastSuccess : styles.toastError)}}>{toast.message}</div>}
    </div>
  );
};

const styles: { [key: string]: React.CSSProperties } = {
    page: { height: '100%', display: 'flex', backgroundColor: '#1e1e1e', color: '#ffffff', padding: '20px', boxSizing: 'border-box' },
    container: { display: 'flex', flex: 1, overflow: 'hidden' },
    leftMenu: { display: 'flex', flexDirection: 'column', paddingRight: '20px', borderRight: '1px solid #444', flexShrink: 0 },
    heading: { fontSize: '2rem', lineHeight: 1.2, color: '#00ffcc', paddingBottom: '20px', marginBottom: '10px' },
    menuButton: { backgroundColor: 'transparent', color: '#ffffff', border: '1px solid #555', padding: '15px 20px', borderRadius: '8px', fontSize: '1.1rem', cursor: 'pointer', transition: 'background-color 0.3s ease, border-color 0.3s ease', textAlign: 'left', width: '240px', marginBottom: '10px' },
    menuButtonActive: { backgroundColor: '#00ffcc20', color: '#00ffcc', border: '1px solid #00ffcc', padding: '15px 20px', borderRadius: '8px', fontSize: '1.1rem', cursor: 'pointer', transition: 'background-color 0.3s ease, border-color 0.3s ease', textAlign: 'left', width: '240px', marginBottom: '10px' },
    rightContent: { flex: 1, overflowY: 'auto', paddingLeft: '25px' },
    section: { backgroundColor: '#2a2a2a', padding: '25px', borderRadius: '10px', boxShadow: '0 4px 8px rgba(0, 0, 0, 0.2)' },
    subHeading: { fontSize: '1.8rem', marginBottom: '20px', color: '#00ffcc', textAlign: 'center', borderBottom: '1px solid #444', paddingBottom: '15px' },
    subHeading4: { fontSize: '1.2rem', marginTop: '20px', marginBottom: '10px', color: '#00ddb3', borderTop: '1px solid #444', paddingTop: '20px' },
    cardsContainer: { display: 'flex', flexDirection: 'column', gap: '20px' },
    card: { backgroundColor: '#333', borderRadius: '8px', padding: '20px', display: 'flex', flexDirection: 'column', boxShadow: '0 2px 4px rgba(0,0,0,0.2)' },
    cardHeader: { display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '15px' },
    cardTitle: { fontSize: '1.4rem', color: '#00ffcc', margin: 0 },
    cardDescription: { color: '#ccc', lineHeight: 1.5, margin: 0 },
    cardBody: { flex: 1, marginBottom: '20px' },
    cardFooter: { textAlign: 'right' },
    statusIndicatorCalibrating: { color: '#ffc107', fontWeight: 'bold' },
    statusIndicatorSuccess: { color: '#28a745', fontWeight: 'bold' },
    statusIndicatorFailed: { color: '#dc3545', fontWeight: 'bold' },
    calibrationProgress: { textAlign: 'center', padding: '20px 0' },
    button: { backgroundColor: '#00ffcc', color: '#1e1e1e', border: 'none', padding: '10px 20px', borderRadius: '5px', fontSize: '1rem', cursor: 'pointer', transition: 'background-color 0.3s ease', fontWeight: 'bold' },
    toastBase: { position: 'fixed', bottom: '20px', left: '50%', transform: 'translateX(-50%)', padding: '12px 24px', borderRadius: '8px', boxShadow: '0 4px 12px rgba(0, 0, 0, 0.4)', zIndex: 1000, fontSize: '1rem', fontWeight: 500, backdropFilter: 'blur(5px)' },
    toastSuccess: { backgroundColor: 'rgba(40, 167, 69, 0.85)', color: '#ffffff', border: '1px solid rgba(40, 167, 69, 1)' },
    toastError: { backgroundColor: 'rgba(220, 53, 69, 0.85)', color: '#ffffff', border: '1px solid rgba(220, 53, 69, 1)' },
    formGroup: { marginBottom: '15px', display: 'flex', flexDirection: 'column', alignItems: 'flex-start' },
    input: { width: '100%', padding: '8px', backgroundColor: '#333', border: '1px solid #555', borderRadius: '4px', color: '#fff', marginTop: '5px', boxSizing: 'border-box' },
    radioGroup: { display: 'flex', gap: '20px', marginTop: '5px' },
    matrixContainer: { display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '5px', backgroundColor: '#1e1e1e', padding: '10px', borderRadius: '4px' },
    matrixCell: { backgroundColor: '#2a2a2a', padding: '5px', textAlign: 'center', borderRadius: '4px' },
    vectorContainer: { display: 'flex', gap: '10px', backgroundColor: '#1e1e1e', padding: '10px', borderRadius: '4px' },
    vectorItem: { display: 'flex', gap: '5px', alignItems: 'center' },
    vectorLabel: { color: '#00ddb3', fontWeight: 'bold' },
};

export default Settings;