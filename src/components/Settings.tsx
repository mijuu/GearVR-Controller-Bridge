import React, { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
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
  madgwick_beta: number;
  orientation_smoothing_factor: number;
  local_earth_mag_field: number;
  mag_calibration: MagCalibration;
  gyro_calibration: GyroCalibration;
}

// --- New split config types ---
interface MouseConfig {
  mode: 'AirMouse' | 'Touchpad';
  touchpad_sensitivity: number;
  touchpad_acceleration: number;
  touchpad_acceleration_threshold: number;
  air_mouse_fov: number;
  air_mouse_activation_threshold: number;
}

interface KeymapConfig {
  trigger: string | null;
  home: string | null;
  back: string | null;
  volume_up: string | null;
  volume_down: string | null;
  touchpad: string | null;
}

interface SettingsProps {
  onBack: () => void;
}

type ActiveMenu = 'calibration' | 'controller' | 'mouse' | 'keymap' | 'language';
type CalibrationStatus = 'idle' | 'calibrating' | 'success' | 'failed';
type ToastType = 'success' | 'error';

// --- Reusable UI Components (unchanged) ---

const Slider: React.FC<{
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (value: number) => void;
  onAfterChange: () => void;
  precision?: number;
}> = ({ label, value, min, max, step, onChange, onAfterChange, precision = 4 }) => (
    <div style={styles.formGroup}>
        <label style={styles.sliderLabel}>
            {label}: <span style={styles.sliderValue}>{value.toFixed(precision)}</span>
        </label>
        <input
            type="range"
            min={min}
            max={max}
            step={step}
            value={value}
            onChange={(e) => onChange(parseFloat(e.target.value))}
            onMouseUp={onAfterChange}
            onTouchEnd={onAfterChange}
            style={styles.slider}
        />
    </div>
);

const Switch: React.FC<{
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  onLabel?: string;
  offLabel?: string;
}> = ({ label, checked, onChange, }) => {
    const switchSliderStyle: React.CSSProperties = {
        ...styles.switchSlider,
        backgroundColor: checked ? '#00cc99' : '#555',
    };

    const switchKnobStyle: React.CSSProperties = {
        ...styles.switchKnob,
        transform: checked ? 'translateX(26px)' : 'translateX(0px)',
    };

    return (
        <div style={styles.formGroupRow}>
            <label style={styles.switchLabel}>{label}</label>
            <div style={styles.switchContainer} onClick={() => onChange(!checked)}>
                <div style={styles.switch}>
                    <div style={switchSliderStyle} />
                    <div style={switchKnobStyle} />
                </div>
            </div>
        </div>
    );
};


// --- Sub-components (unchanged) ---

const CalibrationCard: React.FC<any> = ({ title, description, status, calibrationStep, onStart, t }) => {
    const renderStatus = () => {
        switch (status) {
            case 'calibrating': return <div style={styles.statusIndicatorCalibrating}>{t('settings.calibration.mag.calibrating')}</div>;
            case 'success': return <div style={styles.statusIndicatorSuccess}>{t('settings.calibration.mag.success')}</div>;
            case 'failed': return <div style={styles.statusIndicatorFailed}>{t('settings.calibration.mag.failed')}</div>;
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
                        <p>{t(calibrationStep || 'settings.calibration.mag.step')}</p>
                    </div>
                ) : (
                    <p style={styles.cardDescription}>{description}</p>
                )}
            </div>
            <div style={styles.cardFooter}>
                <button onClick={status === 'calibrating' ? undefined : onStart} disabled={status === 'calibrating'} style={styles.button}>
                    {status === 'success' || status === 'failed' ? t('settings.calibration.mag.recalibrate') : t('settings.calibration.mag.start')}
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
const Settings: React.FC<SettingsProps> = ({ onBack }) => {
  const { t, i18n } = useTranslation();
  const [magCalibrationStatus, setMagCalibrationStatus] = useState<CalibrationStatus>('idle');
  const [gyroCalibrationStatus, setGyroCalibrationStatus] = useState<CalibrationStatus>('idle');
  const [magCalibrationStep, setMagCalibrationStep] = useState('');
  const [gyroCalibrationStep, setGyroCalibrationStep] = useState('');
  const [controllerConfig, setControllerConfig] = useState<ControllerConfig | null>(null);
  const [mouseConfig, setMouseConfig] = useState<MouseConfig | null>(null);
  const [keymapConfig, setKeymapConfig] = useState<KeymapConfig | null>(null);
  const [activeMenu, setActiveMenu] = useState<ActiveMenu>('calibration');
  const [toast, setToast] = useState<{ message: string; type: ToastType } | null>(null);
  const [capturingKeyFor, setCapturingKeyFor] = useState<string | null>(null);
  const [hoveredReset, setHoveredReset] = useState<string | null>(null);

  const factoryDefaultMappings: { [key: string]: string | null } = {
    trigger: 'Left',
    home: '',
    back: 'Backspace',
    volume_up: 'Volume up',
    volume_down: 'Volume down',
    touchpad: 'Right',
  };

  const showToast = (message: string, type: ToastType = 'success') => {
    setToast({ message, type });
    setTimeout(() => setToast(null), 3000);
  };

  const changeLanguage = async (lang: string) => {
    try {
      await i18n.changeLanguage(lang);
      await invoke('set_current_language', { language: lang });
      showToast(t('settings.toast.languageChanged'), 'success');
    } catch (error) {
      console.error('Failed to change language', error);
      showToast(t('settings.toast.languageChangeFailed'), 'error');
    }
  };

  const handleControllerConfigChange = useCallback((field: string, value: any) => {
    if (!controllerConfig) return;
    const newConfig = { ...controllerConfig, [field]: parseFloat(value) };
    setControllerConfig(newConfig);
    invoke('set_controller_config', { config: newConfig })
        .then(() => showToast(t('settings.toast.controllerSaved'), 'success'))
        .catch(err => {
            showToast(t('settings.toast.saveFailed'), 'error');
            console.error('Failed to save controller config:', err);
        });
  }, [controllerConfig]);

  const handleMouseConfigChange = useCallback((field: keyof MouseConfig, value: any) => {
    if (!mouseConfig) return;
    const newConfig = { ...mouseConfig, [field]: value };
    setMouseConfig(newConfig);
    invoke('set_mouse_config', { config: newConfig })
        .then(() => showToast(t('settings.toast.mouseSaved'), 'success'))
        .catch(err => {
            showToast(t('settings.toast.saveFailed'), 'error');
            console.error('Failed to save mouse config:', err);
        });
  }, [mouseConfig]);

  const handleKeymapChange = useCallback((key: keyof KeymapConfig, value: string | null) => {
    if (!keymapConfig) return;
    const newConfig = { ...keymapConfig, [key]: value };
    setKeymapConfig(newConfig);
    invoke('set_keymap_config', { config: newConfig })
        .then(() => showToast(t('settings.toast.keymapSaved'), 'success'))
        .catch(err => {
            showToast(t('settings.toast.saveFailed'), 'error');
            console.error('Failed to save keymap config:', err);
        });
  }, [keymapConfig]);

  useEffect(() => {
    const handleCapture = (keyString: string | null) => {
        if (!capturingKeyFor) return;
        handleKeymapChange(capturingKeyFor as keyof KeymapConfig, keyString);
        setCapturingKeyFor(null);
    };

    const handleKeyDown = (event: KeyboardEvent) => {
        event.preventDefault();
        event.stopPropagation();

        if (event.key === 'Escape') {
            if (capturingKeyFor) {
                const defaultValue = factoryDefaultMappings[capturingKeyFor.toLowerCase()];
                return handleCapture(defaultValue ?? null);
            }
            return handleCapture(null);
        }

        const parts = [];
        if (event.ctrlKey) parts.push('Control');
        if (event.altKey) parts.push('Alt');
        if (event.shiftKey) parts.push('Shift');
        if (event.metaKey) parts.push('Meta');
        
        const keyName = event.key;
        if (!['Control', 'Alt', 'Shift', 'Meta'].includes(keyName)) {
            parts.push(keyName);
        }
        
        if (parts.length > (event.ctrlKey ? 1 : 0) + (event.altKey ? 1 : 0) + (event.shiftKey ? 1 : 0) + (event.metaKey ? 1 : 0)) {
            handleCapture(parts.join('+'));
        }
    };

    const handleMouseDown = (event: MouseEvent) => {
        event.preventDefault();
        event.stopPropagation();

        let buttonName = null;
        switch (event.button) {
            case 0: buttonName = 'Left'; break;
            case 1: buttonName = 'Middle'; break;
            case 2: buttonName = 'Right'; break;
        }

        if (buttonName) {
            handleCapture(buttonName);
        }
    };

    const preventDefault = (e: Event) => e.preventDefault();

    if (capturingKeyFor) {
        window.addEventListener('keydown', handleKeyDown, true);
        window.addEventListener('mousedown', handleMouseDown, true);
        window.addEventListener('contextmenu', preventDefault, true);
    } 

    return () => {
        window.removeEventListener('keydown', handleKeyDown, true);
        window.removeEventListener('mousedown', handleMouseDown, true);
        window.removeEventListener('contextmenu', preventDefault, true);
    };
  }, [capturingKeyFor, handleKeymapChange]);

  useEffect(() => {
    const unlistenMagStep = listen<string>('mag-calibration-step', (event) => setMagCalibrationStep(event.payload));
    const unlistenGyroStep = listen<string>('gyro-calibration-step', (event) => setGyroCalibrationStep(event.payload));
    const unlistenMagFinished = listen<boolean>('mag-calibration-finished', (event) => {
        setMagCalibrationStatus(event.payload ? 'success' : 'failed');
        invoke<ControllerConfig>('get_controller_config').then(setControllerConfig);
    });
    const unlistenGyroFinished = listen<boolean>('gyro-calibration-finished', (event) => {
        setGyroCalibrationStatus(event.payload ? 'success' : 'failed');
        invoke<ControllerConfig>('get_controller_config').then(setControllerConfig);
    });

    if (controllerConfig === null) {
        invoke<ControllerConfig>('get_controller_config').then(setControllerConfig).catch(console.error);
    }
    if (mouseConfig === null) {
        invoke<MouseConfig>('get_mouse_config').then(setMouseConfig).catch(console.error);
    }
    if (keymapConfig === null) {
        invoke<KeymapConfig>('get_keymap_config').then(setKeymapConfig).catch(console.error);
    }

    return () => {
      unlistenMagStep.then(f => f());
      unlistenGyroStep.then(f => f());
      unlistenMagFinished.then(f => f());
      unlistenGyroFinished.then(f => f());
    };
  }, [magCalibrationStatus]);

  const handleStartMagCalibration = async () => {
    try {
      setMagCalibrationStatus('calibrating');
      setMagCalibrationStep('settings.calibration.mag.step');
      await invoke('start_mag_calibration_wizard');
    } catch (error) {
      console.error('Failed to start mag calibration:', error);
      setMagCalibrationStatus('failed');
    }
  };

  const handleStartGyroCalibration = async () => {
    setGyroCalibrationStatus('calibrating');
    setGyroCalibrationStep('settings.calibration.gyro.calibrating');
    try {
      await invoke('start_gyro_calibration');
      setGyroCalibrationStatus('success');
      invoke<ControllerConfig>('get_controller_config').then(setControllerConfig);
    } catch (error) {
      console.error('Failed to start gyro calibration:', error);
      setGyroCalibrationStatus('failed');
    }
  };

  const handleResetControllerConfig = async () => {
    try {
      const config = await invoke<ControllerConfig>('reset_controller_config');
      setControllerConfig(config);
      showToast(t('settings.toast.controllerReset'), 'success');
    } catch (err) {
      showToast(t('settings.toast.resetFailed'), 'error');
      console.error('Failed to reset controller config:', err);
    }
  };

  const handleResetMouseConfig = async () => {
    try {
      const config = await invoke<MouseConfig>('reset_mouse_config');
      setMouseConfig(config);
      showToast(t('settings.toast.mouseReset'), 'success');
    } catch (err) {
      showToast(t('settings.toast.resetFailed'), 'error');
      console.error('Failed to reset mouse config:', err);
    }
  };

  const handleResetKeymapConfig = async () => {
    try {
      const config = await invoke<KeymapConfig>('reset_keymap_config');
      setKeymapConfig(config);
      showToast(t('settings.toast.keymapReset'), 'success');
    } catch (err) {
      showToast(t('settings.toast.resetFailed'), 'error');
      console.error('Failed to reset keymap config:', err);
    }
  };

  const renderContent = () => {
    switch (activeMenu) {
      case 'calibration':
        return controllerConfig && (
          <div style={styles.section}>
            <div style={styles.subHeadingContainer}>
                <h3 style={styles.subHeading}>{t('settings.calibration.title')}</h3>
            </div>
            <div style={styles.cardsContainer}>
                <CalibrationCard t={t} title={t('settings.calibration.mag.title')} description={t('settings.calibration.mag.description')} status={magCalibrationStatus} calibrationStep={magCalibrationStatus === 'calibrating' ? t(magCalibrationStep) : undefined} onStart={handleStartMagCalibration} />
                <CalibrationCard t={t} title={t('settings.calibration.gyro.title')} description={t('settings.calibration.gyro.description')} status={gyroCalibrationStatus} calibrationStep={gyroCalibrationStatus === 'calibrating' ? t(gyroCalibrationStep) : undefined} onStart={handleStartGyroCalibration} />

                <div style={styles.card}>
                  <h4 style={styles.subHeading4}>{t('settings.calibration.gyroData')}</h4>
                  <VectorDisplay vector={controllerConfig.gyro_calibration.zero_bias} />

                  <h4 style={styles.subHeading4}>{t('settings.calibration.magData')}</h4>
                  <label>{t('settings.calibration.hardIronBias')}</label>
                  <VectorDisplay vector={controllerConfig.mag_calibration.hard_iron_bias} />
                  <label style={{marginTop: '10px'}}>{t('settings.calibration.softIronMatrix')}</label>
                  <MatrixDisplay matrix={controllerConfig.mag_calibration.soft_iron_matrix} />
                </div>
            </div>
          </div>
        );
      case 'controller':
        return controllerConfig && (
            <div style={styles.section}>
                <div style={styles.subHeadingContainer}>
                    <h3 style={styles.subHeading}>{t('settings.controller.title')}</h3>
                </div>
                <Slider
                    label={t('settings.controller.sensorLowPass')}
                    min={0} max={1} step={0.01} value={controllerConfig.sensor_low_pass_alpha}
                    onChange={(v) => setControllerConfig({ ...controllerConfig, sensor_low_pass_alpha: v })}
                    onAfterChange={() => handleControllerConfigChange('sensor_low_pass_alpha', controllerConfig.sensor_low_pass_alpha)}
                    precision={2}
                />
                <Slider
                    label={t('settings.controller.deltaT')}
                    min={0} max={1} step={0.01} value={controllerConfig.delta_t_smoothing_alpha}
                    onChange={(v) => setControllerConfig({ ...controllerConfig, delta_t_smoothing_alpha: v })}
                    onAfterChange={() => handleControllerConfigChange('delta_t_smoothing_alpha', controllerConfig.delta_t_smoothing_alpha)}
                    precision={2}
                />
                <Slider
                    label={t('settings.controller.madgwickBeta')}
                    min={0} max={1} step={0.01} value={controllerConfig.madgwick_beta}
                    onChange={(v) => setControllerConfig({ ...controllerConfig, madgwick_beta: v })}
                    onAfterChange={() => handleControllerConfigChange('madgwick_beta', controllerConfig.madgwick_beta)}
                    precision={2}
                />
                <Slider
                    label={t('settings.controller.orientationSmoothing')}
                    min={0} max={1} step={0.01} value={controllerConfig.orientation_smoothing_factor}
                    onChange={(v) => setControllerConfig({ ...controllerConfig, orientation_smoothing_factor: v })}
                    onAfterChange={() => handleControllerConfigChange('orientation_smoothing_factor', controllerConfig.orientation_smoothing_factor)}
                    precision={2}
                />
                <Slider
                    label={t('settings.controller.earthMagField')}
                    min={20} max={70} step={1} value={controllerConfig.local_earth_mag_field}
                    onChange={(v) => setControllerConfig({ ...controllerConfig, local_earth_mag_field: v })}
                    onAfterChange={() => handleControllerConfigChange('local_earth_mag_field', controllerConfig.local_earth_mag_field)}
                    precision={0}
                />
                <div style={styles.resetButtonContainer}>
                    <button 
                        onClick={handleResetControllerConfig} 
                        style={hoveredReset === 'controller' ? { ...styles.resetButton, ...styles.resetButtonHover } : styles.resetButton}
                        onMouseEnter={() => setHoveredReset('controller')}
                        onMouseLeave={() => setHoveredReset(null)}
                    >
                        {t('settings.controller.reset')}
                    </button>
                </div>
            </div>
          );
      case 'mouse':
        return mouseConfig && (
            <div style={styles.section}>
                <div style={styles.subHeadingContainer}>
                    <h3 style={styles.subHeading}>{t('settings.mouse.title')}</h3>
                </div>
                <Switch
                    label={t('settings.mouse.enableAirMouse')}
                    checked={mouseConfig.mode === 'AirMouse'}
                    onChange={(isChecked) => handleMouseConfigChange('mode', isChecked ? 'AirMouse' : 'Touchpad')}
                />
                <Slider
                    label={t('settings.mouse.touchpadSensitivity')}
                    min={1} max={1000} step={1} value={mouseConfig.touchpad_sensitivity}
                    onChange={(v) => setMouseConfig({...mouseConfig, touchpad_sensitivity: v})}
                    onAfterChange={() => handleMouseConfigChange('touchpad_sensitivity', mouseConfig.touchpad_sensitivity)}
                    precision={0}
                />
                <Slider
                    label={t('settings.mouse.touchpadAcceleration')}
                    min={0} max={10} step={0.1} value={mouseConfig.touchpad_acceleration}
                    onChange={(v) => setMouseConfig({...mouseConfig, touchpad_acceleration: v})}
                    onAfterChange={() => handleMouseConfigChange('touchpad_acceleration', mouseConfig.touchpad_acceleration)}
                    precision={1}
                />
                <Slider
                    label={t('settings.mouse.touchpadAccelThreshold')}
                    min={0} max={0.01} step={0.0001} value={mouseConfig.touchpad_acceleration_threshold}
                    onChange={(v) => setMouseConfig({...mouseConfig, touchpad_acceleration_threshold: v})}
                    onAfterChange={() => handleMouseConfigChange('touchpad_acceleration_threshold', mouseConfig.touchpad_acceleration_threshold)}
                    precision={4}
                />
                <Slider
                    label={t('settings.mouse.airMouseFov')}
                    min={10} max={180} step={1} value={mouseConfig.air_mouse_fov}
                    onChange={(v) => setMouseConfig({...mouseConfig, air_mouse_fov: v})}
                    onAfterChange={() => handleMouseConfigChange('air_mouse_fov', mouseConfig.air_mouse_fov)}
                    precision={0}
                />
                <Slider
                    label={t('settings.mouse.airMouseActivationThreshold')}
                    min={0} max={20} step={0.5} value={mouseConfig.air_mouse_activation_threshold}
                    onChange={(v) => setMouseConfig({...mouseConfig, air_mouse_activation_threshold: v})}
                    onAfterChange={() => handleMouseConfigChange('air_mouse_activation_threshold', mouseConfig.air_mouse_activation_threshold)}
                    precision={1}
                />
                <div style={styles.resetButtonContainer}>
                    <button 
                        onClick={handleResetMouseConfig} 
                        style={hoveredReset === 'mouse' ? { ...styles.resetButton, ...styles.resetButtonHover } : styles.resetButton}
                        onMouseEnter={() => setHoveredReset('mouse')}
                        onMouseLeave={() => setHoveredReset(null)}
                    >
                        {t('settings.mouse.reset')}
                    </button>
                </div>
            </div>
          );
      case 'keymap':
        return keymapConfig && (
            <div style={styles.section}>
                <div style={styles.subHeadingContainer}>
                    <h3 style={styles.subHeading}>{t('settings.keymap.title')}</h3>
                </div>
                <h4 style={styles.subHeading4}>{t('settings.keymap.description')}</h4>
                {Object.entries(keymapConfig).map(([key, value]) => (
                <div style={styles.formGroupRow} key={key}>
                  <label style={styles.keymapLabel}>{t(`settings.keymap.keys.${key}`)}</label>
                  <button 
                    onClick={() => setCapturingKeyFor(key)}
                    style={capturingKeyFor === key ? styles.keymapButtonCapturing : styles.keymapButton}
                  >
                    {capturingKeyFor === key ? t('settings.keymap.capturing') : (value || t('settings.keymap.none'))}
                  </button>
                </div>
              ))}
              <div style={styles.resetButtonContainer}>
                  <button 
                    onClick={handleResetKeymapConfig} 
                    style={hoveredReset === 'keymap' ? { ...styles.resetButton, ...styles.resetButtonHover } : styles.resetButton}
                    onMouseEnter={() => setHoveredReset('keymap')}
                    onMouseLeave={() => setHoveredReset(null)}
                  >
                    {t('settings.keymap.reset')}
                  </button>
              </div>
            </div>
        );
      case 'language':
        return (
            <div style={styles.section}>
                <div style={styles.subHeadingContainer}>
                    <h3 style={styles.subHeading}>{t('settings.language.title')}</h3>
                </div>
                <div style={styles.formGroupRow}>
                    <label style={styles.switchLabel}>{t('settings.language.select')}</label>
                    <select style={styles.select} value={i18n.language} onChange={(e) => changeLanguage(e.target.value)}>
                        <option value="en">English</option>
                        <option value="zh">中文</option>
                    </select>
                </div>
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
                <h2 style={styles.heading}>{t('settings.title')}</h2>
                <button style={activeMenu === 'calibration' ? styles.menuButtonActive : styles.menuButton} onClick={() => setActiveMenu('calibration')}>{t('settings.menu.calibration')}</button>
                <button style={activeMenu === 'controller' ? styles.menuButtonActive : styles.menuButton} onClick={() => setActiveMenu('controller')}>{t('settings.menu.controller')}</button>
                <button style={activeMenu === 'mouse' ? styles.menuButtonActive : styles.menuButton} onClick={() => setActiveMenu('mouse')}>{t('settings.menu.mouse')}</button>
                <button style={activeMenu === 'keymap' ? styles.menuButtonActive : styles.menuButton} onClick={() => setActiveMenu('keymap')}>{t('settings.menu.keymap')}</button>
                <button style={activeMenu === 'language' ? styles.menuButtonActive : styles.menuButton} onClick={() => setActiveMenu('language')}>{t('settings.menu.language')}</button>
                <button style={{...styles.menuButton, marginTop: 'auto'}} onClick={onBack}>{t('settings.back')}</button>
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
    page: { height: '100%', display: 'flex', backgroundColor: '#1e1e1e', color: '#ffffff', boxSizing: 'border-box' },
    container: { display: 'flex', flex: 1, overflow: 'hidden' },
    leftMenu: { display: 'flex', flexDirection: 'column', padding: '20px', borderRight: '1px solid #444', flexShrink: 0 },
    heading: { fontSize: '2rem', lineHeight: 1.2, color: '#00ffcc', paddingBottom: '20px', marginBottom: '10px' },
    menuButton: { backgroundColor: 'transparent', color: '#ffffff', border: '1px solid #555', padding: '15px 20px', borderRadius: '8px', fontSize: '1.1rem', cursor: 'pointer', transition: 'background-color 0.3s ease, border-color 0.3s ease', textAlign: 'left', width: '240px', marginBottom: '10px' },
    menuButtonActive: { backgroundColor: '#00ffcc20', color: '#00ffcc', border: '1px solid #00ffcc', padding: '15px 20px', borderRadius: '8px', fontSize: '1.1rem', cursor: 'pointer', transition: 'background-color 0.3s ease, border-color 0.3s ease', textAlign: 'left', width: '240px', marginBottom: '10px' },
    rightContent: { flex: 1, overflowY: 'auto', padding: '25px', backgroundColor: '#2a2a2a', boxShadow: '0 4px 8px rgba(0, 0, 0, 0.2)' },
    section: { padding: '0', borderRadius: '0', boxShadow: 'none', backgroundColor: 'transparent' },
    subHeadingContainer: { display: 'flex', justifyContent: 'space-between', alignItems: 'center', height: '52px', marginBottom: '20px', borderBottom: '1px solid #444', paddingBottom: '15px' },
    subHeading: { fontSize: '1.8rem', color: '#00ffcc', textAlign: 'left', borderBottom: 'none', paddingBottom: '0', margin: 0 },
    resetButton: {
        backgroundColor: '#dc3545',
        color: '#ffffff',
        border: 'none',
        padding: '8px 15px',
        borderRadius: '5px',
        fontSize: '0.9rem',
        cursor: 'pointer',
        transition: 'background-color 0.3s ease',
        fontWeight: 'bold',
    },
    resetButtonHover: {
        backgroundColor: '#c82333'
    },
    resetButtonContainer: {
        borderTop: '1px solid #444',
        marginTop: '20px',
        paddingTop: '20px',
        display: 'flex',
        justifyContent: 'flex-end',
    },
    subHeading4: { fontSize: '1.2rem', marginTop: '20px', marginBottom: '10px', color: '#00ddb3' },
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
    toastBase: { position: 'fixed', top: '20px', left: '50%', transform: 'translateX(-50%)', padding: '12px 24px', borderRadius: '8px', boxShadow: '0 4px 12px rgba(0, 0, 0, 0.4)', zIndex: 1000, fontSize: '1rem', fontWeight: 500, backdropFilter: 'blur(5px)' },
    toastSuccess: { backgroundColor: 'rgba(40, 167, 69, 0.85)', color: '#ffffff', border: '1px solid rgba(40, 167, 69, 1)' },
    toastError: { backgroundColor: 'rgba(220, 53, 69, 0.85)', color: '#ffffff', border: '1px solid rgba(220, 53, 69, 1)' },
    formGroup: { marginBottom: '20px', display: 'flex', flexDirection: 'column', alignItems: 'flex-start' },
    formGroupRow: { marginBottom: '20px', display: 'flex', alignItems: 'center', justifyContent: 'space-between' },
    input: { width: '100%', padding: '8px', backgroundColor: '#333', border: '1px solid #555', borderRadius: '4px', color: '#fff', marginTop: '5px', boxSizing: 'border-box' },
    matrixContainer: { display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '5px', backgroundColor: '#1e1e1e', padding: '10px', borderRadius: '4px' },
    matrixCell: { backgroundColor: '#2a2a2a', padding: '5px', textAlign: 'center', borderRadius: '4px' },
    vectorContainer: { display: 'flex', gap: '10px', backgroundColor: '#1e1e1e', padding: '10px', borderRadius: '4px' },
    vectorItem: { display: 'flex', gap: '5px', alignItems: 'center' },
    vectorLabel: { color: '#00ddb3', fontWeight: 'bold' },
    sliderLabel: { display: 'flex', justifyContent: 'space-between', width: '100%', color: '#eee', fontSize: '1rem' },
    sliderValue: { color: '#00ffcc', fontWeight: 'bold' },
    slider: {
        width: '100%',
        appearance: 'none',
        WebkitAppearance: 'none',
        height: '8px',
        background: '#444',
        borderRadius: '5px',
        outline: 'none',
        opacity: 0.9,
        transition: 'opacity .2s',
        cursor: 'pointer',
        marginTop: '10px',
    },
    switchLabel: {
        fontSize: '1rem',
        color: '#eee',
    },
    switchContainer: {
        display: 'flex',
        alignItems: 'center',
        cursor: 'pointer',
    },
    switch: {
        position: 'relative',
        display: 'inline-block',
        width: '60px',
        height: '34px',
        margin: '0 10px',
    },
    switchSlider: {
        position: 'absolute',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        transition: '.4s',
        borderRadius: '34px',
    },
    switchKnob: {
        position: 'absolute',
        content: '""',
        height: '26px',
        width: '26px',
        left: '4px',
        bottom: '4px',
        backgroundColor: 'white',
        transition: '.4s',
        borderRadius: '50%',
    },
    switchText: {
        fontWeight: 'bold',
        color: '#00ffcc',
        width: '70px',
        textAlign: 'center',
    },
    keymapLabel: {
        color: '#eee',
        fontSize: '1rem',
    },
    keymapButton: {
        backgroundColor: '#333',
        color: '#fff',
        border: '1px solid #555',
        padding: '8px 15px',
        borderRadius: '5px',
        fontSize: '1rem',
        cursor: 'pointer',
        transition: 'background-color 0.3s ease, border-color 0.3s ease',
        minWidth: '150px',
        textAlign: 'center',
    },
    keymapButtonCapturing: {
        backgroundColor: '#00ffcc',
        color: '#1e1e1e',
        border: '1px solid #00ffcc',
        padding: '8px 15px',
        borderRadius: '5px',
        fontSize: '1rem',
        cursor: 'pointer',
        transition: 'background-color 0.3s ease, border-color 0.3s ease',
        minWidth: '150px',
        textAlign: 'center',
        fontWeight: 'bold',
    },
    select: {
        border: '1px solid #555',
        padding: '8px 15px',
        borderRadius: '5px',
        fontSize: '1rem',
        cursor: 'pointer',
        minWidth: '150px',
    }
};

export default Settings;
