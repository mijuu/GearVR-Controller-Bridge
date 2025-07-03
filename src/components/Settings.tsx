import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

interface SettingsProps {
  onBackToController: (view: 'controller') => void;
}

const Settings: React.FC<SettingsProps> = ({ onBackToController }) => {
  const [calibrationStatus, setCalibrationStatus] = useState('idle'); // idle, calibrating, success, failed
  const [calibrationStep, setCalibrationStep] = useState('');

  useEffect(() => {
    const unlistenStep = listen<string>('calibration-step', (event) => {
      setCalibrationStep(event.payload);
    });

    const unlistenFinished = listen<boolean>('calibration-finished', (event) => {
      setCalibrationStatus(event.payload ? 'success' : 'failed');
    });

    return () => {
      unlistenStep.then(f => f());
      unlistenFinished.then(f => f());
    };
  }, []);

  const handleStartCalibration = async () => {
    try {
      setCalibrationStatus('calibrating');
      await invoke('start_calibration_wizard');
    } catch (error) {
      console.error('Failed to start calibration:', error);
      setCalibrationStatus('failed');
    }
  };

  return (
    <div style={styles.container}>
      <h2 style={styles.heading}>校准设置</h2>

      <div style={styles.section}>
        <h3 style={styles.subHeading}>磁力计校准</h3>
        {calibrationStatus === 'idle' && (
          <button onClick={handleStartCalibration} style={styles.button}>
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
            <button onClick={handleStartCalibration} style={styles.button}>
              重新校准
            </button>
          </div>
        )}
      </div>

      <button onClick={() => onBackToController('controller')} style={styles.backButton}>
        回到控制器界面
      </button>
    </div>
  );
};

const styles: { [key: string]: React.CSSProperties } = {
  container: {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    justifyContent: 'center',
    height: '100%',
    backgroundColor: '#1e1e1e',
    color: '#ffffff',
    padding: '20px',
    boxSizing: 'border-box',
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
    width: '80%',
    maxWidth: '500px',
    textAlign: 'center',
    boxShadow: '0 4px 8px rgba(0, 0, 0, 0.2)',
  },
  subHeading: {
    fontSize: '1.8rem',
    marginBottom: '20px',
    color: '#00ffcc',
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
  buttonStop: {
    backgroundColor: '#ff4d4d',
    color: '#ffffff',
    border: 'none',
    padding: '12px 25px',
    borderRadius: '5px',
    fontSize: '1.1rem',
    cursor: 'pointer',
    transition: 'background-color 0.3s ease',
    fontWeight: 'bold',
    marginTop: '10px',
  },
  recordingStatus: {
    marginTop: '15px',
    color: '#cccccc',
  },
  backButton: {
    backgroundColor: '#4a4a4a',
    color: '#ffffff',
    border: 'none',
    padding: '10px 20px',
    borderRadius: '5px',
    fontSize: '1rem',
    cursor: 'pointer',
    transition: 'background-color 0.3s ease',
    marginTop: '30px',
  },
};

export default Settings;
