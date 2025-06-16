import { useEffect, useRef } from 'react';

export interface LogMessage {
  level: string;
  message: string;
  timestamp: string;
  repeatCount?: number; // 添加重复计数字段
}

interface LogViewerProps {
  logs: LogMessage[];
  onClearLogs?: () => void;
}

// 格式化时间戳函数
const formatTimestamp = (timestamp: string): string => {
  try {
    const date = new Date(timestamp);
    
    // 检查日期是否有效
    if (isNaN(date.getTime())) {
      return timestamp; // 如果无法解析，则返回原始时间戳
    }
    
    // 格式化为 HH:MM:SS.mmm
    const hours = date.getHours().toString().padStart(2, '0');
    const minutes = date.getMinutes().toString().padStart(2, '0');
    const seconds = date.getSeconds().toString().padStart(2, '0');
    const milliseconds = date.getMilliseconds().toString().padStart(3, '0');
    
    return `${hours}:${minutes}:${seconds}.${milliseconds}`;
  } catch (error) {
    console.error("Error formatting timestamp:", error);
    return timestamp; // 出错时返回原始时间戳
  }
};

function LogViewer({ logs, onClearLogs }: LogViewerProps) {
  const logContentRef = useRef<HTMLDivElement>(null);

  // 自动滚动到底部
  useEffect(() => {
    if (logContentRef.current) {
      logContentRef.current.scrollTop = logContentRef.current.scrollHeight;
    }
  }, [logs]);

  const getLevelColor = (level: string) => {
    switch (level.toLowerCase()) {
      case 'error':
        return '#dc3545'; // 鲜明的红色，表示错误
      case 'warn':
        return '#fd7e14'; // 醒目的橙色，表示警告
      case 'info':
        return '#0d6efd'; // 清晰的蓝色，表示信息
      case 'debug':
        return '#6c757d'; // 柔和的灰色，表示调试信息
      case 'trace':
        return '#198754'; // 柔和的绿色，表示跟踪信息
      default:
        return '#6c757d'; // 默认使用灰色
    }
  };

  return (
    <div className="log-viewer" style={{ marginTop: '10px' }}>
      <div className="log-header">
        <div className="log-title">
          <span>日志查看器 {logs.length > 0 ? `(${logs.length})` : ''}</span>
        </div>
        <button 
          onClick={onClearLogs} 
          className="clear-logs-button"
          disabled={logs.length === 0}
        >
          清除日志
        </button>
      </div>
      <div 
        className="log-content" 
        ref={logContentRef}
      >
        <div className="log-entries">
          {logs.map((log, index) => (
          <div key={index} className="log-entry">
            <span className="log-timestamp">{formatTimestamp(log.timestamp)}</span>
            <span 
              className="log-level"
              style={{ color: getLevelColor(log.level) }}
            >
              [{log.level}]
            </span>
            <span className="log-message">{log.message}</span>
            {log.repeatCount && log.repeatCount > 1 && (
              <span className="log-repeat-count" style={{ marginLeft: '8px', color: '#888', fontSize: '0.85em' }}>
                (x{log.repeatCount})
              </span>
            )}
          </div>
        ))}
        {logs.length === 0 && (
          <div className="no-logs">暂无日志</div>
        )}
      </div>
      </div>
      <style>{`
        .log-viewer {
          background-color: #f8f9fa;
          border: 1px solid #ddd;
          border-radius: 4px;
          margin: 0;
          max-height: 300px;
          display: flex;
          flex-direction: column;
          box-shadow: 0 2px 4px rgba(0, 0, 0, 0.05);
          height: 100%;
        }

        .log-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 0.5rem 1rem;
          background-color: #4a4a4a;
          color: white;
          border-radius: 4px 4px 0 0;
        }

        .log-title {
          display: flex;
          align-items: center;
          gap: 8px;
          font-weight: 500;
        }

        .log-content {
          overflow-y: auto;
          padding: 0.75rem;
          font-family: 'Consolas', 'Monaco', monospace;
          font-size: 0.85rem;
          color: #333;
          background-color: #fff;
          flex: 1;
          height: 100%;
          box-sizing: border-box;
          display: flex;
          flex-direction: column;
        }

        .log-entries {
          flex: 1;
          overflow-y: auto;
        }

        .clear-logs-button {
          padding: 0.25rem 0.5rem;
          background-color: rgba(255, 255, 255, 0.2);
          border: 1px solid rgba(255, 255, 255, 0.3);
          border-radius: 3px;
          cursor: pointer;
          font-size: 0.85rem;
          color: white;
          transition: background-color 0.2s;
        }

        .clear-logs-button:hover {
          background-color: rgba(255, 255, 255, 0.3);
        }

        .clear-logs-button:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }

        .log-content::-webkit-scrollbar {
          width: 8px;
        }

        .log-content::-webkit-scrollbar-track {
          background: #f1f1f1;
          border-radius: 4px;
        }

        .log-content::-webkit-scrollbar-thumb {
          background: #c1c1c1;
          border-radius: 4px;
        }

        .log-content::-webkit-scrollbar-thumb:hover {
          background: #a8a8a8;
        }

        .log-entry {
          padding: 0.3rem 0.2rem;
          display: flex;
          gap: 0.5rem;
          align-items: flex-start;
          border-bottom: 1px solid #f0f0f0;
          transition: background-color 0.2s ease;
        }

        .log-entry:hover {
          background-color: #f8f9fa;
        }

        .log-entry:last-child {
          border-bottom: none;
        }

        .log-timestamp {
          color: #888;
          white-space: nowrap;
          font-size: 0.8rem;
        }

        .log-level {
          white-space: nowrap;
          font-weight: 500;
        }

        .log-message {
          word-break: break-all;
          flex: 1;
        }

        .no-logs {
          color: #888;
          text-align: center;
          padding: 1.5rem;
          font-style: italic;
        }
      `}</style>
    </div>
  );
}

export default LogViewer;