import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from '@tauri-apps/api/core';
import { Canvas } from "@react-three/fiber";
import ControllerModel from "../ControllerModel/ControllerModel";
import './ControllerView.css';

export interface ControllerState {
    timestamp: number;
    buttons: {
        trigger: boolean;
        home: boolean;
        back: boolean;
        volume_up: boolean;
        volume_down: boolean;
        touchpad: boolean;
    };
    touchpad: {
        touched: boolean;
        x: number;
        y: number;
    };
    orientation: [number, number, number, number];
    accelerometer: [number, number, number];
    gyroscope: [number, number, number];
    magnetometer: [number, number, number];
    temperature: number;
}

export default function ControllerStatus() {
    const [state, setState] = useState<ControllerState | null>(null);
    const [battery_level, setBatteryLevel] = useState<number | null>(null);
    const [isConnected, setIsConnected] = useState(true);
    const reconnectTimeoutRef = useRef<any>(null);

    // Effect for listeners
    useEffect(() => {
        const stopReconnecting = () => {
            if (reconnectTimeoutRef.current) {
                clearTimeout(reconnectTimeoutRef.current);
                reconnectTimeoutRef.current = null;
            }
        };

        const setupListeners = async () => {
            const unlistenState = await listen<ControllerState>(
                "controller-state",
                (event) => {
                    setIsConnected(true);
                    stopReconnecting();
                    setState(event.payload);
                }
            );

            const unlistenLostConnection = await listen<void>('device-lost-connection', () => {
                setIsConnected(false);
                setBatteryLevel(null); // Battery level is uncertain

                stopReconnecting(); // Clear any previous loop

                const tryReconnect = async () => {
                    console.log("Attempting to reconnect to the device...");
                    try {
                        await invoke('reconnect_to_device');
                        // On success, the 'controller-state' event should fire,
                        // which will call stopReconnecting() and break the loop.
                    } catch (err) {
                        console.error("Reconnect attempt failed:", err);
                        // If the attempt fails, schedule the next one.
                        // The timeout will be cancelled by stopReconnecting if a connection is established.
                        reconnectTimeoutRef.current = setTimeout(tryReconnect, 3000);
                    }
                };

                // Start the first attempt.
                tryReconnect();
            });

            return () => {
                unlistenState();
                unlistenLostConnection();
            };
        };

        const unlistenPromise = setupListeners();

        return () => {
            unlistenPromise.then(unlisten => unlisten && unlisten());
            stopReconnecting();
        };
    }, []); // Empty dependency array, runs only once.

    // Effect for polling battery level
    useEffect(() => {
        if (!isConnected) return;

        const updateBatteryLevel = async () => {
            try {
                const batteryLevel = await invoke('get_battery_level') as number;
                setBatteryLevel(batteryLevel);
            } catch (error) {
                console.error('Failed to get battery level:', error);
            }
        };
        updateBatteryLevel();

        const intervalId = setInterval(updateBatteryLevel, 5000);

        return () => {
            clearInterval(intervalId);
        };
    }, [isConnected]);

    // const handleRescanClick = () => {
    //     window.location.reload();
    // };

    return (
        <div className="controller-status">
            {!isConnected && (
                <div className="connection-lost-overlay">
                    <div className="connection-lost-toast">
                        <h2>连接丢失</h2>
                        <p>正在尝试重新连接，请按键任意键唤醒您的控制器。</p>
                        {/* <p>如果长时间无响应，您可以点击<span className="rescan-link" onClick={handleRescanClick}>重新扫描</span>查找设备。</p> */}
                    </div>
                </div>
            )}

            {state && (
                <>
                    <div className="top-section">
                        <div className="model-view">
                            <Canvas>
                                <ControllerModel state={state} />
                            </Canvas>
                        </div>
                        <div className="right-panel">
                            <div className="touchpad-container">
                                <div className="touchpad-display">
                                    <div
                                        className="touch-indicator"
                                        style={{
                                            display: state.touchpad.touched ? "block" : "none",
                                            left: `${state.touchpad.x * 100}%`,
                                            top: `${state.touchpad.y * 100}%`,
                                        }}
                                    ></div>
                                    <div className="touchpad-coords">
                                        <span>X: {state.touchpad.x.toFixed(2)} </span>
                                        <span>Y: {state.touchpad.y.toFixed(2)}</span>
                                    </div>
                                </div>
                            </div>
                            <div className="buttons-container">
                                <div className="button-grid">
                                    <div className={`button-indicator ${state.buttons.trigger ? "active" : ""}`}>
                                        <span>Trigger</span>
                                    </div>
                                    <div className={`button-indicator ${state.buttons.home ? "active" : ""}`}>
                                        <span>Home</span>
                                    </div>
                                    <div className={`button-indicator ${state.buttons.back ? "active" : ""}`}>
                                        <span>Back</span>
                                    </div>
                                    <div className={`button-indicator ${state.buttons.touchpad ? "active" : ""}`}>
                                        <span>Touchpad</span>
                                    </div>
                                    <div className={`button-indicator ${state.buttons.volume_up ? "active" : ""}`}>
                                        <span>Vol+</span>
                                    </div>
                                    <div className={`button-indicator ${state.buttons.volume_down ? "active" : ""}`}>
                                        <span>Vol-</span>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                    <div className="bottom-section">
                        <div className="device-status">
                            <div className="battery-status">
                                <span>电池: </span>
                                <div className="battery-bar">
                                    <div
                                        className="battery-level"
                                        style={{ width: `${battery_level}%` }}
                                    ></div>
                                </div>
                                <span>{battery_level || '? '}%</span>
                            </div>
                            <div className="temperature-status">
                                <span>温度: </span>
                                <span>{state.temperature.toFixed(1)}°C</span>
                            </div>
                        </div>
                    </div>
                </>
            )}
        </div>
    );
}