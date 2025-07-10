import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import MainView from "./components/MainView";
import StatusBar from "./components/StatusBar";
import LogViewer from "./components/LogViewer";
import ControllerView from "./components/ControllerView/ControllerView";
import Settings from "./components/Settings";
import { LogMessage } from "./components/LogViewer";
import "./App.css";

export type AppView = 'controller' | 'settings';

function App() {
  // 状态管理
  const [isConnected, setIsConnected] = useState(false);
  const [isCheckingConnection, setIsCheckingConnection] = useState(true);
  const [connectedDevice, setConnectedDevice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showLogs, setShowLogs] = useState(false);
  const [logs, setLogs] = useState<LogMessage[]>([]);
  const logListenerRef = useRef<(() => void) | null>(null);
  const [activeView, setActiveView] = useState<AppView>('controller');

  // 清除日志的函数
  const clearLogs = () => {
    setLogs([]);
  };

  // Check initial connection status on mount
  useEffect(() => {
    const checkInitialConnection = async () => {
      setIsCheckingConnection(true);
      try {
        // NOTE: This requires a backend command `get_connection_status` that returns
        // an object: { is_connected: boolean, device_name: string | null }
        const status = await invoke<{ is_connected: boolean, device_name: string | null }>('get_connection_status');
        if (status.is_connected) {
          setIsConnected(true);
          setConnectedDevice(status.device_name);
        }
      } catch (err) {
        console.error("Failed to check initial connection status:", err);
      } finally {
        setIsCheckingConnection(false);
      }
    };
    checkInitialConnection();
  }, []);

  // 监听连接状态变化
  useEffect(() => {
    const unlistenConnect = listen("device-connected", (event) => {
      const deviceName = (event.payload as { name?: string })?.name;
      setIsConnected(true);
      setConnectedDevice(deviceName || null);
      setError(null);
      setActiveView('controller'); // Switch to controller view on connect
    });

    const unlistenDisconnect = listen("device-lost-connection", () => {
      setIsConnected(false);
      setConnectedDevice(null);
    });

    const unlistenError = listen("device-error", (event) => {
      const errorMessage = (event.payload as { message: string }).message;
      setError(errorMessage);
    });

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

  const renderContent = () => {
    if (isCheckingConnection) {
      return <div style={{ textAlign: 'center', paddingTop: '4rem', color: '#888' }}>检查连接状态...</div>;
    }

    if (!isConnected) {
      return <MainView />;
    }

    switch (activeView) {
      case 'settings':
        return <Settings onBack={() => setActiveView('controller')} />;
      case 'controller':
      default:
        return <ControllerView />;
    }
  };

  return (
    <div className="app">
      <header className="app-header">
        <h1>GearVR Controller Bridge</h1>
        {error && <div className="error-banner">{error}</div>}
      </header>

      <main className="app-content">
        {renderContent()}
        {showLogs && (
          <div className="log-overlay">
            <LogViewer logs={logs} onClearLogs={clearLogs} />
          </div>
        )}
      </main>

      <footer className="app-footer">
        <StatusBar 
          isConnected={isConnected} 
          deviceName={connectedDevice || undefined}
          showLogs={showLogs}
          setShowLogs={setShowLogs}
          onViewChange={(view) => setActiveView(view)}
        />
      </footer>

      <style>{`
        .app {
          display: flex;
          flex-direction: column;
          height: 100vh;
          background-color: #121212;
          color: #ffffff;
        }

        .app-header {
          display: none;
          padding: 1rem;
          background-color: #1a1a1a;
          color: #00ffcc;
          border-bottom: 1px solid #333;
        }

        .app-header h1 {
          margin: 0;
          font-size: 1.5rem;
          font-weight: 500;
        }

        .error-banner {
          margin-top: 0.5rem;
          padding: 0.5rem;
          background-color: #ff0033;
          color: white;
          border-radius: 4px;
          font-size: 0.9rem;
        }

        .app-content {
          flex: 1;
          overflow: hidden;
          position: relative;
          height: calc(100vh - 120px); /* 减去header和footer的高度 */
        }

        .app-footer {
          /* border-top: 1px solid #333; */
          /* background-color: #1a1a1a; */
        }

        .log-overlay {
          position: fixed;
          left: 0;
          right: 0;
          bottom: 40px;
          top: 40px;
          background-color: rgba(18, 18, 18, 0.98);
          backdrop-filter: blur(5px);
          z-index: 1000;
          padding: 0 1rem;
          display: flex;
          flex-direction: column;
          pointer-events: all;
          transition: all 0.3s ease;
          box-shadow: 0 -4px 6px rgba(0, 0, 0, 0.3);
          border-top: 1px solid rgba(0, 255, 204, 0.1);
          animation: slideIn 0.3s ease;
        }

        @keyframes slideIn {
          from {
            opacity: 0;
            transform: translateY(20px);
          }
          to {
            opacity: 1;
            transform: translateY(0);
          }
        }

        .log-overlay .log-viewer {
          flex: 1;
          max-height: none !important;
          margin: 0 !important;
          height: 100%;
        }

        .log-overlay .log-content {
          max-height: none !important;
          height: calc(100% - 40px) !important;
        }
      `}</style>
    </div>
  );
}

export default App;
