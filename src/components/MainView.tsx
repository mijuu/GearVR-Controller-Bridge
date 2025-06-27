import React, { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import ControllerStatus, { ControllerState } from './ControllerStatus/ControllerStatus';

interface BluetoothDevice {
  name: string;
  address: string;
  id: string;
  is_connected: boolean;
}

const MainView: React.FC = () => {
  const [status, setStatus] = useState<'searching' | 'found' | 'connecting' | 'connected' | 'failed'>('searching');
  const [device, setDevice] = useState<BluetoothDevice | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showController, setShowController] = useState(false);

  // Text animation variants
  const textVariants = {
    hidden: { y: 20, opacity: 0 },
    visible: { y: 0, opacity: 1 },
    exit: { y: -20, opacity: 0 }
  };

  // Start device search on mount
  useEffect(() => {    
    // Listen for device found event
    const deviceFoundUnlisten = listen<BluetoothDevice>('device-found', (event) => {
      // Skip if already found or same device
      if (status !== 'searching' || (device && device.id === event.payload.id)) {
        return;
      }
      

      const newDevice = event.payload;
      setDevice(newDevice);
      setStatus('found');
      
      // Show "found" status for 2 seconds before connecting
      setTimeout(() => {
        setStatus('connecting');
        connectToDevice(newDevice.id);
      }, 1500);
    });

    // Listen for connection events
    const connectUnlisten = listen<{id: string}>('device-connected', () => {
      setTimeout(() => {
        setStatus('connected');
      }, 1500);
    });

    const payloadUnlisten = listen<ControllerState>("controller-state", (event) => {   
      if (event.payload) {
        setStatus('connected');
        setTimeout(() => {
          setShowController(true);
        }, 1500);
      }
    });

    searchDevices();

    return () => {
      // Stop scanning when component unmounts
      invoke('stop_scan').catch(console.error);

      deviceFoundUnlisten.then(f => f());
      connectUnlisten.then(f => f());
      payloadUnlisten.then(f => f());
    };
  }, []);
  
  const searchDevices = async () => {
    try {
      setStatus('searching');        
      setError(null);
      await invoke('start_scan');
    } catch (err) {
      console.error('scan', err);
      setError(`搜索失败: ${err}`);
    }
  };
  
  const connectToDevice = async (deviceId: string) => {
    try {
      setStatus('connecting');
      setError(null);
      await invoke('connect_to_device', { deviceId });
    } catch (err) {
      const errorMessage = typeof err === 'string' ? err 
                        : err instanceof Error ? err.message
                        : '未知错误';
      
      if (errorMessage.includes('Peer removed pairing information')) {
        setError('检查到设备已被重置，请在系统设置中选择忽略此设备后，重新尝试连接');
      } else if (errorMessage.includes('the Bluetooth device isn\'t connected: unreachable')) {
        setError('检查到设备已被重置，请在系统设置中选择删除此设备后，重新尝试连接')
      } else {
        setError(`连接失败: ${errorMessage}`);
      }
      setStatus('failed');
    }
  };

  if (showController && device) {
    return <ControllerStatus />;
  }

  return (
    <div className="main-view" style={{
      background: '#121212',
      color: '#00ffcc',
      height: '100%',
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      gap: '2rem',
      position: 'relative',
      overflow: 'hidden'
    }}>
      {/* Optimized pulse animation */}
      {status !== 'failed' && (
        <div style={{
          position: 'fixed',
          width: '300px',
          height: '300px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          willChange: 'transform',
          pointerEvents: 'none'
        }}>
          {Array.from({ length: 3 }).map((_, i) => (
            <motion.div
              key={i}
              style={{
                width: '100%',
                height: '100%',
                borderRadius: '50%',
                border: '2px solid #00ffcc',
                position: 'absolute',
                boxShadow: '0 0 10px #00ffcc',
                willChange: 'transform, opacity'
              }}
              variants={{
                initial: { 
                  scale: 0.8, 
                  opacity: 0.7,
                  transformOrigin: 'center'
                },
                animate: { 
                  scale: 1.5,
                  opacity: 0,
                  transition: {
                    duration: 2,
                    delay: i * 0.5,
                    repeat: Infinity,
                    ease: "easeOut",
                    repeatDelay: 0.5
                  }
                }
              }}
              initial="initial"
              animate="animate"
            />
          ))}
        </div>
      )}

      {/* Status messages */}
      <AnimatePresence mode="wait">
        {status === 'searching' && (
          <motion.div
            key="searching"
            variants={textVariants}
            initial="hidden"
            animate="visible"
            exit="exit"
            style={{ zIndex: 1, textAlign: 'center' }}
          >
            <h1 style={{ fontSize: '2rem', marginBottom: '1rem' }}>搜索设备中...</h1>
            <p>正在搜索附近的蓝牙设备</p>
          </motion.div>
        )}

        {status === 'found' && device && (
          <motion.div
            key="found"
            variants={textVariants}
            initial="hidden"
            animate="visible"
            exit="exit"
            style={{ zIndex: 1, textAlign: 'center' }}
          >
            <h1 style={{ fontSize: '2rem', marginBottom: '1rem' }}>找到设备</h1>
            <p>{device.name || '未知设备'}</p>
          </motion.div>
        )}

        {status === 'connecting' && (
          <motion.div
            key="connecting"
            variants={textVariants}
            initial="hidden"
            animate="visible"
            exit="exit"
            style={{ zIndex: 1, textAlign: 'center' }}
          >
            <h1 style={{ fontSize: '2rem', marginBottom: '1rem' }}>连接中...</h1>
            <p>{'连接 ' + device?.name}</p>
          </motion.div>
        )}

        {status === 'connected' && (
          <motion.div
            key="connected"
            variants={textVariants}
            initial="hidden"
            animate="visible"
            exit="exit"
            style={{ zIndex: 1, textAlign: 'center' }}
          >
            <h1 style={{ fontSize: '2rem', marginBottom: '1rem' }}>连接成功!</h1>
            <p>进入控制模式</p>
          </motion.div>
        )}

        {status === 'failed' && (
          <motion.div
            key="failed"
            variants={textVariants}
            initial="hidden"
            animate="visible"
            exit="exit"
            style={{ zIndex: 1, textAlign: 'center' }}
          >
            <h1 style={{ fontSize: '2rem', marginBottom: '1rem' }}>连接失败!</h1>
            <div className="rescan-button" onClick={ searchDevices }>
              <span>重新搜索</span>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
      {/* Error message */}
      {error && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          style={{
            position: 'absolute',
            bottom: '2rem',
            background: '#ff0033',
            color: 'white',
            padding: '1rem',
            borderRadius: '4px',
            zIndex: 1
          }}
        >
          {error}
        </motion.div>
      )}
      <style>{`
        .rescan-button {
            background-color: #333;
            color: #00ffcc;
            padding: 0.6rem;
            border-radius: 4px;
            text-align: center;
            cursor: pointer;
            user-select: none;
            transition: .5s;
        }
        .rescan-button:hover {
            background-color: #00ffcc;
            color: #333;
        }
      `}</style>
    </div>
  );
};

export default MainView;