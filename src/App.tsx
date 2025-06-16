import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import DeviceList from "./components/DeviceList";
import StatusBar from "./components/StatusBar";
import LogViewer from "./components/LogViewer";
import { LogMessage } from "./components/LogViewer";
import "./App.css";

function App() {
  // 状态管理
  const [isConnected, setIsConnected] = useState(false);
  const [connectedDevice, setConnectedDevice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showLogs, setShowLogs] = useState(false); // 控制日志视图的显示/隐藏
  const [logs, setLogs] = useState<LogMessage[]>([]); // 日志状态
  const logListenerRef = useRef<(() => void) | null>(null);

  // 清除日志的函数
  const clearLogs = () => {
    setLogs([]);
  };

  // 监听连接状态变化
  useEffect(() => {
    // 监听连接成功事件
    const unlistenConnect = listen("device-connected", (event) => {
      const deviceName = (event.payload as { name?: string })?.name;
      setIsConnected(true);
      setConnectedDevice(deviceName || null);
      setError(null);
    });

    // 监听断开连接事件
    const unlistenDisconnect = listen("device-disconnected", () => {
      setIsConnected(false);
      setConnectedDevice(null);
    });

    // 监听错误事件
    const unlistenError = listen("device-error", (event) => {
      const errorMessage = (event.payload as { message: string }).message;
      setError(errorMessage);
    });

    // 清理监听器
    return () => {
      unlistenConnect.then(unlisten => unlisten());
      unlistenDisconnect.then(unlisten => unlisten());
      unlistenError.then(unlisten => unlisten());
    };
  }, []);

  // 设置日志监听器
  useEffect(() => {
    let isSubscribed = true;

    const setupLogListener = async () => {
      if (!isSubscribed) return;

      try {
        // 如果已经有监听器，先移除它
        if (logListenerRef.current) {
          await logListenerRef.current();
          logListenerRef.current = null;
        }

        // 设置新的监听器
        const unlistenLog = await listen('log-message', (event) => {
          if (!isSubscribed) return;
          
          const logMessage = event.payload as LogMessage;
          setLogs(prevLogs => {
            const lastLog = prevLogs[prevLogs.length - 1];
            
            if (lastLog && 
                lastLog.message === logMessage.message && 
                lastLog.level === logMessage.level &&
                Math.abs(new Date(lastLog.timestamp).getTime() - new Date(logMessage.timestamp).getTime()) < 500) {
              const updatedLogs = [...prevLogs];
              updatedLogs[updatedLogs.length - 1] = {
                ...lastLog,
                repeatCount: (lastLog.repeatCount || 1) + 1
              };
              return updatedLogs;
            }
            
            return [...prevLogs, { ...logMessage, repeatCount: 1 }];
          });
        });

        logListenerRef.current = unlistenLog;
      } catch (error) {
        console.error('Failed to setup log listener:', error);
      }
    };

    setupLogListener();
    
    return () => {
      isSubscribed = false;
      if (logListenerRef.current) {
        logListenerRef.current();
        logListenerRef.current = null;
      }
    };
  }, []);

  return (
    <div className="app">
      <header className="app-header">
        <h1>GearVR Controller Bridge</h1>
        {error && <div className="error-banner">{error}</div>}
      </header>

      <main className="app-content">
        <DeviceList />
        {showLogs && (
          <div className="log-section">
            <div className="log-viewer-container">
              <LogViewer logs={logs} onClearLogs={clearLogs} />
            </div>
          </div>
        )}
      </main>

      <footer className="app-footer">
        <StatusBar 
          isConnected={isConnected} 
          deviceName={connectedDevice || undefined}
          showLogs={showLogs}
          setShowLogs={setShowLogs}
        />
      </footer>

      <style>{`
        .app {
          display: flex;
          flex-direction: column;
          height: 100vh;
          background-color: #f5f5f5;
        }

        .app-header {
          padding: 1rem;
          background-color: #1a1a1a;
          color: white;
        }

        .app-header h1 {
          margin: 0;
          font-size: 1.5rem;
          font-weight: 500;
        }

        .error-banner {
          margin-top: 0.5rem;
          padding: 0.5rem;
          background-color: #dc3545;
          color: white;
          border-radius: 4px;
          font-size: 0.9rem;
        }

        .app-content {
          flex: 1;
          overflow-y: auto;
          padding: 1rem;
        }

        .app-footer {
          border-top: 1px solid #ddd;
          background-color: white;
        }

        .log-section {
          margin-top: 1rem;
          border-top: 1px solid #ddd;
          padding-top: 1rem;
        }

        .log-viewer-container {
          transition: opacity 0.3s, max-height 0.3s;
          overflow: hidden;
        }
      `}</style>
    </div>
  );
}

export default App;