import React, { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';

interface StatusBarProps {
  isConnected: boolean;
  deviceName?: string;
  showLogs: boolean;
  setShowLogs: (show: boolean) => void;
}

const StatusBar: React.FC<StatusBarProps> = ({ isConnected, deviceName, showLogs, setShowLogs }) => {
  const [batteryLevel, setBatteryLevel] = useState<number | null>(null);
  const [lastDataReceived, setLastDataReceived] = useState<Date | null>(null);

  // 监听控制器数据事件
  useEffect(() => {
    if (!isConnected) {
      setBatteryLevel(null);
      return;
    }

    const unlistenPromise = listen('controller-data', (event) => {
      // 更新最后接收数据的时间
      setLastDataReceived(new Date());
      
      // 这里可以解析电池电量信息
      // 实际实现需要根据GearVR控制器的数据格式来解析
      // 目前只是一个占位符
      const data = event.payload as { data: number[] };
      if (data && Array.isArray(data.data) && data.data.length > 0) {
        // 假设电池电量在某个特定位置
        // 实际实现需要根据控制器协议来解析
        // setBatteryLevel(calculateBatteryLevel(data.data));
      }
    });

    return () => {
      unlistenPromise.then(unlisten => unlisten());
    };
  }, [isConnected]);

  // 格式化最后接收数据的时间
  const formatLastReceived = () => {
    if (!lastDataReceived) return '无数据';
    
    const now = new Date();
    const diff = now.getTime() - lastDataReceived.getTime();
    
    if (diff < 1000) return '刚刚';
    if (diff < 60000) return `${Math.floor(diff / 1000)}秒前`;
    if (diff < 3600000) return `${Math.floor(diff / 60000)}分钟前`;
    
    return lastDataReceived.toLocaleTimeString();
  };

  return (
    <div className="status-bar">
      <div className="status-left">
        <div className="connection-status">
          <div className={`status-indicator ${isConnected ? 'connected' : 'disconnected'}`}></div>
          <span>
            {isConnected
              ? `已连接: ${deviceName || '未知设备'}`
              : '未连接'}
          </span>
        </div>
        
        {isConnected && (
          <>
            <div className="battery-status">
              <span className="status-label">电池:</span>
              <span className="status-value">
                {batteryLevel !== null ? `${batteryLevel}%` : '未知'}
              </span>
            </div>
            
            <div className="data-status">
              <span className="status-label">最后数据:</span>
              <span className="status-value">{formatLastReceived()}</span>
            </div>
          </>
        )}
      </div>
      
      <div className="status-right">
        <div 
          className="log-toggle-link" 
          onClick={() => setShowLogs(!showLogs)}
        >
          {showLogs ? '隐藏日志' : '显示日志'}
        </div>
      </div>

      <style>{`
        .status-bar {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 0.5rem 1rem;
          background-color: #f8f9fa;
          border-top: 1px solid #ddd;
          font-size: 0.9rem;
        }

        .status-left {
          display: flex;
          align-items: center;
        }

        .status-right {
          display: flex;
          align-items: center;
        }

        .connection-status {
          display: flex;
          align-items: center;
          margin-right: 1.5rem;
        }

        .status-indicator {
          width: 10px;
          height: 10px;
          border-radius: 50%;
          margin-right: 0.5rem;
        }

        .status-indicator.connected {
          background-color: #28a745;
        }

        .status-indicator.disconnected {
          background-color: #dc3545;
        }

        .battery-status,
        .data-status {
          margin-right: 1.5rem;
        }

        .status-label {
          color: #666;
          margin-right: 0.25rem;
        }

        .status-value {
          font-weight: 500;
        }

        .log-toggle-link {
          color: #0275d8;
          cursor: pointer;
          font-size: 0.9rem;
          transition: color 0.2s;
          user-select: none;
        }

        .log-toggle-link:hover {
          color: #014c8c;
          text-decoration: underline;
        }
      `}</style>
    </div>
  );
};

export default StatusBar;