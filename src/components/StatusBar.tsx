import React from 'react';
import { AppView } from '../App';

interface StatusBarProps {
  isConnected: boolean;
  deviceName?: string;
  showLogs: boolean;
  setShowLogs: (show: boolean) => void;
  onViewChange: (view: AppView) => void;
}

const StatusBar: React.FC<StatusBarProps> = ({ isConnected, deviceName, showLogs, setShowLogs, onViewChange }) => {
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
      </div>
      
      <div className="status-right">
        <div 
          className="log-toggle-link" 
          onClick={() => setShowLogs(!showLogs)}
        >
          {showLogs ? '隐藏日志' : '显示日志'}
        </div>
        <div 
          className="settings-link" 
          onClick={() => onViewChange('settings')}
          style={{ marginLeft: '1rem' }}
        >
          设置
        </div>
      </div>

      <style>{`
        .status-bar {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 0.5rem 1rem;
          color: #666;
          /* background-color: #f8f9fa; */
          /* border-top: 1px solid #ddd; */
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

        .log-toggle-link, .settings-link {
          cursor: pointer;
          font-size: 0.9rem;
          transition: color 0.2s;
          user-select: none;
        }

        .log-toggle-link:hover, .settings-link:hover {
          text-decoration: underline;
        }
      `}</style>
    </div>
  );
};

export default StatusBar;