import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// Define Bluetooth device type
export interface BluetoothDevice {
  name: string;
  address: string;
  id: string;
  rssi: number;
  is_paired: boolean;
  is_connected: boolean;
}

const DeviceList: React.FC = () => {
  // State management
  const [devices, setDevices] = useState<BluetoothDevice[]>([]);
  const [isScanning, setIsScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  // Set up event listeners
  useEffect(() => {
    // Listen for device discovery events
    const deviceFoundUnlisten = listen<BluetoothDevice>('device-found', (event) => {
      const newDevice = event.payload;
      setDevices((currentDevices) => {
        // Check if device already exists
        const exists = currentDevices.some(device => device.id === newDevice.id);
        if (exists) {
          // Update existing device
          return currentDevices.map(device => 
            device.id === newDevice.id ? newDevice : device
          );
        } else {
          // Add new device
          return [...currentDevices, newDevice];
        }
      });
    });

    // Update device status
    const updateConnectedUnlisten = listen<BluetoothDevice>('device-connected', (event) => {
      const { id } = event.payload;
      setDevices((currentDevices) => {
          // Update existing device
          return currentDevices.map(device => 
            device.id === id ? {...device , is_connected: true} : device
          );
      });
    });
    
    // Update device status
    const updateDisconnectedUnlisten = listen<BluetoothDevice>('device-disconnected', (event) => {
      const { id } = event.payload;
      setDevices((currentDevices) => {
          // Update existing device
          return currentDevices.map(device => 
            device.id === id ? {...device , is_connected: false} : device
          );
      });
    });

    // Listen for scan completion event
    const scanCompleteUnlisten = listen('scan-complete', () => {
      setIsScanning(false);
    });
    
    // Listen for scan error event
    const scanErrorUnlisten = listen<string>('scan-error', (event) => {
      setError(`扫描错误: ${event.payload}`);
      setIsScanning(false);
    });
    
    // Cleanup function
    return () => {
      deviceFoundUnlisten.then(unlisten => unlisten());
      updateConnectedUnlisten.then(unlisten => unlisten());
      updateDisconnectedUnlisten.then(unlisten => unlisten());
      scanCompleteUnlisten.then(unlisten => unlisten());
      scanErrorUnlisten.then(unlisten => unlisten());
    };
  }, []);

  // Scan devices in real-time
  const scanDevicesRealtime = async () => {
    try {
      setIsScanning(true);
      setError(null);
      // Clear device list
      setDevices([]);
      
      // Start real-time scanning
      await invoke('scan_devices_realtime', {
        durationSecs: 5,
      });
      
    } catch (err) {
      setError(`扫描失败: ${err}`);
      setIsScanning(false);
    }
  };

  const [connectingDeviceId, setConnectingDeviceId] = useState<string | null>(null);
  // Connect to device
  const connectToDevice = async (deviceId: string) => {
    try {
      setConnectingDeviceId(deviceId);
      setError(null);
      await invoke('connect_to_device', { deviceId });
      // 连接成功后的逻辑（如果需要）
    } catch (err) {
      const errorMessage = typeof err === 'string' ? err 
                        : err instanceof Error ? err.message
                        : '未知错误';
      
      if (errorMessage.includes('Peer removed pairing information')) {
        setError('检查到设备已被重置，请在系统设置中选择忽略此设备后，重新尝试连接');
      } else {
        setError(`连接失败: ${errorMessage}`);
      }
    } finally {
      setConnectingDeviceId(null);
    }
  };

  // Disconnect
  const disconnect = async (deviceId: string) => {
    try {
      setError(null);
      await invoke('disconnect', { deviceId });
      setDevices((currentDevices) => {
          // Update existing device
          return currentDevices.map(device => 
            device.id === deviceId ? {...device , is_connected: false } : device
          );
      });
    } catch (err) {
      setError(`断开连接失败: ${err}`);
    }
  };

  // Render signal strength indicator
  const renderSignalStrength = (rssi?: number) => {
    if (!rssi) return '--';
    if (rssi > -50) return '强';
    if (rssi > -70) return '中';
    return '弱';
  };

  return (
    <div className="device-list">
      <div className="controls">
        <button
          onClick={scanDevicesRealtime}
          disabled={isScanning}
          className="button scan-button"
        >
          {isScanning ? (
            <>
              <span className="scanning-text">扫描中</span>
              <span className="scanning-dots">...</span>
            </>
          ) : (
            '扫描设备'
          )}
        </button>
      </div>

      <style>{`
        @keyframes blink {
          0% { opacity: .2; }
          20% { opacity: 1; }
          100% { opacity: .2; }
        }
        
        .scanning-text {
          margin-right: 4px;
        }
        
        .scanning-dots {
          animation: blink 1.4s infinite both;
        }
      `}</style>

      {error && <div className="error-message">{error}</div>}

      <div className="devices">
        {devices.map((device) => (
          <div
            key={device.id}
            className={`device-item ${
              device.is_connected ? 'selected' : ''
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
              {device.is_connected ? (
                <>
                  <button onClick={() => disconnect(device.id)} className="button disconnect-button">
                    断开连接
                  </button>
                </>
              ) : (
                <button
                  onClick={() => connectToDevice(device.id)}
                  disabled={connectingDeviceId === device.id}
                >
                  {connectingDeviceId === device.id ? (
                    <span>连接中...</span>
                  ) : (
                    <span>连接</span>
                  )}
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
          margin-bottom: 0.25rem;
        }

        .device-actions {
          margin-left: 1rem;
        }

        .button {
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