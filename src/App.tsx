import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import DeviceList from "./components/DeviceList";
import StatusBar from "./components/StatusBar";
import "./App.css";

function App() {
  // 状态管理
  const [isConnected, setIsConnected] = useState(false);
  const [connectedDevice, setConnectedDevice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

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

  return (
    <div className="app">
      <header className="app-header">
        <h1>GearVR Controller Bridge</h1>
        {error && <div className="error-banner">{error}</div>}
      </header>

      <main className="app-content">
        <DeviceList />
      </main>

      <footer className="app-footer">
        <StatusBar isConnected={isConnected} deviceName={connectedDevice || undefined} />
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
      `}</style>
    </div>
  );
}

export default App;