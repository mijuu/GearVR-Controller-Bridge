import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

// 定义蓝牙设备类型
interface BluetoothDevice {
  name?: string;
  address: string;
  id: string;
  rssi?: number;
}

const DeviceList: React.FC = () => {
  // 状态管理
  const [devices, setDevices] = useState<BluetoothDevice[]>([]);
  const [isScanning, setIsScanning] = useState(false);
  const [selectedDevice, setSelectedDevice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // 扫描设备
  const scanDevices = async () => {
    try {
      setIsScanning(true);
      setError(null);
      // 清空设备列表
      setDevices([]);
      const foundDevices = await invoke<BluetoothDevice[]>('scan_devices', {
        durationSecs: 5,
      });
      setDevices(foundDevices);
    } catch (err) {
      setError(`扫描失败: ${err}`);
    } finally {
      setIsScanning(false);
    }
  };

  // 连接设备
  const connectToDevice = async (deviceId: string) => {
    try {
      setError(null);
      await invoke('connect_to_device', { deviceId });
      setSelectedDevice(deviceId);
    } catch (err) {
      setError(`连接失败: ${err}`);
    }
  };

  // 断开连接
  const disconnect = async () => {
    try {
      setError(null);
      await invoke('disconnect');
      setSelectedDevice(null);
    } catch (err) {
      setError(`断开连接失败: ${err}`);
    }
  };

  // 渲染信号强度指示器
  const renderSignalStrength = (rssi?: number) => {
    if (!rssi) return '无信号';
    if (rssi > -50) return '强';
    if (rssi > -70) return '中';
    return '弱';
  };

  return (
    <div className="device-list">
      <div className="controls">
        <button
          onClick={scanDevices}
          disabled={isScanning}
          className="scan-button"
        >
          {isScanning ? '扫描中...' : '扫描设备'}
        </button>
      </div>

      {error && <div className="error-message">{error}</div>}

      <div className="devices">
        {devices.map((device) => (
          <div
            key={device.id}
            className={`device-item ${
              selectedDevice === device.id ? 'selected' : ''
            }`}
          >
            <div className="device-info">
              <div className="device-name">
                {device.name || '未知设备'}
              </div>
              <div className="device-address">
                MAC: {device.address}
                {device.address === '00:00:00:00:00:00' && (
                  <span className="device-id">
                    <br />
                    ID: {device.id}
                  </span>
                )}
              </div>
              <div className="signal-strength">
                信号: {renderSignalStrength(device.rssi)}
              </div>
            </div>
            <div className="device-actions">
              {selectedDevice === device.id ? (
                <button onClick={disconnect} className="disconnect-button">
                  断开连接
                </button>
              ) : (
                <button
                  onClick={() => connectToDevice(device.id)}
                  className="connect-button"
                >
                  连接
                </button>
              )}
            </div>
          </div>
        ))}
        {devices.length === 0 && !isScanning && (
          <div className="no-devices">
            未发现设备
          </div>
        )}
      </div>

      <style>{`
        .device-list {
          padding: 1rem;
          max-width: 600px;
          margin: 0 auto;
        }

        .controls {
          margin-bottom: 1rem;
        }

        .scan-button {
          padding: 0.5rem 1rem;
          background-color: #007bff;
          color: white;
          border: none;
          border-radius: 4px;
          cursor: pointer;
        }

        .scan-button:disabled {
          background-color: #ccc;
          cursor: not-allowed;
        }

        .error-message {
          color: #dc3545;
          padding: 0.5rem;
          margin-bottom: 1rem;
          border: 1px solid #dc3545;
          border-radius: 4px;
          background-color: #f8d7da;
        }

        .device-item {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 1rem;
          margin-bottom: 0.5rem;
          border: 1px solid #ddd;
          border-radius: 4px;
          background-color: white;
        }

        .device-item.selected {
          border-color: #007bff;
          background-color: #f8f9fa;
        }

        .device-info {
          flex: 1;
        }

        .device-name {
          font-weight: bold;
          margin-bottom: 0.25rem;
        }

        .device-address {
          color: #666;
          font-size: 0.9rem;
          margin-bottom: 0.25rem;
        }

        .signal-strength {
          color: #28a745;
          font-size: 0.9rem;
        }

        .device-actions {
          margin-left: 1rem;
        }

        .connect-button,
        .disconnect-button {
          padding: 0.375rem 0.75rem;
          border: none;
          border-radius: 4px;
          cursor: pointer;
        }

        .connect-button {
          background-color: #28a745;
          color: white;
        }

        .disconnect-button {
          background-color: #dc3545;
          color: white;
        }

        .no-devices {
          text-align: center;
          padding: 2rem;
          color: #666;
          border: 1px dashed #ddd;
          border-radius: 4px;
        }
      `}</style>
    </div>
  );
};

export default DeviceList;