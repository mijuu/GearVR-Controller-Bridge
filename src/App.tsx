import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import MainView from "./components/MainView";
import StatusBar from "./components/StatusBar";
import LogViewer from "./components/LogViewer";
import ControllerView from "./components/ControllerView/ControllerView";
import Settings from "./components/Settings";
import { LogMessage } from "./components/LogViewer";
import "./App.css";

type AppView = 'main' | 'controller' | 'settings';

function App() {
  // 状态管理
  const [isConnected, setIsConnected] = useState(false);
  const [connectedDevice, setConnectedDevice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showLogs, setShowLogs] = useState(false); // 控制日志视图的显示/隐藏
  const [logs, setLogs] = useState<LogMessage[]>([]); // 日志状态
  const logListenerRef = useRef<(() => void) | null>(null);
  const [currentView, setCurrentView] = useState<AppView>('main'); // 新增：当前视图状态

  // 清除日志的函数
  const clearLogs = () => {
    setLogs([]);
  };

  // 处理控制器连接成功，切换到控制器状态视图
  const handleControllerConnected = () => {
    setCurrentView('controller');
  };

  // 处理视图切换
  const handleViewChange = (view: AppView) => {
    setCurrentView(view);
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
        {currentView === 'main' && <MainView onControllerConnected={handleControllerConnected} />}
        {currentView === 'controller' && <ControllerView />}
        {currentView === 'settings' && <Settings onBackToController={handleViewChange} />}
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
          onViewChange={handleViewChange}
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