import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { Canvas } from "@react-three/fiber";
import Controller3DView from "../Controller3DView/Controller3DView";
import "./ControllerStatus.css";

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
    orientation: {
        x: number;
        y: number;
        z: number;
        w: number;
    };
    accelerometer: {
        x: number;
        y: number;
        z: number;
    };
    gyroscope: {
        x: number;
        y: number;
        z: number;
    };
    magnetometer: {
        x: number;
        y: number;
        z: number;
    };
    temperature: number;
}

export default function ControllerStatus() {
    const [state, setState] = useState<ControllerState | null>(null);
    const [battery_level, setBatteryLevel] = useState<number | null>(null);

    useEffect(() => {
        const setupListener = async () => {
            const unlisten = await listen<number>(
                "battery-level",
                (event) => {
                    setBatteryLevel(event.payload);
                }
                );
            return unlisten;
        };

        const unlistenPromise = setupListener();

        return () => {
            unlistenPromise.then((unlisten) => unlisten());
        };
    }, []);

    useEffect(() => {
        const setupListener = async () => {
            const unlisten = await listen<ControllerState>(
                "controller-state",
                (event) => {
                    setState(event.payload);
                }
            );
            return unlisten;
        };

        const unlistenPromise = setupListener();

        return () => {
            unlistenPromise.then((unlisten) => unlisten());
        };
    }, []);

    if (!state) {
        return <div className="controller-status">等待控制器数据...</div>;
    }

    return (
        <div className="controller-status">
            <h2>控制器状态</h2>
            <div className="controller-layout">
                <div className="controller-3d-view">
                    <Canvas>
                        <Controller3DView state={state} />
                    </Canvas>
                </div>
            </div>
            <div className="controller-data">
        
                <div className="section">
                    <h3>按钮状态</h3>
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

                <div className="section">
                    <h3>触摸板</h3>
                    <div className="touchpad-display">
                        <div 
                            className="touch-indicator"
                            style={{
                                display: state.touchpad.touched ? "block" : "none",
                                left: `${state.touchpad.x * 100}%`,
                                top: `${state.touchpad.y * 100}%`,
                            }}
                        ></div>
                        <span>X: {state.touchpad.x.toFixed(2)}</span>
                        <span>Y: {state.touchpad.y.toFixed(2)}</span>
                    </div>
                </div>

                <div className="section">
                    <h3>传感器数据</h3>
                    <div className="sensor-grid">
                        <div className="sensor-data">
                            <h4>方向 (四元数)</h4>
                            <div>X: {state.orientation.x.toFixed(4)}</div>
                            <div>Y: {state.orientation.y.toFixed(4)}</div>
                            <div>Z: {state.orientation.z.toFixed(4)}</div>
                            <div>W: {state.orientation.w.toFixed(4)}</div>
                        </div>
                        <div className="sensor-data">
                            <h4>加速度 (m/s²)</h4>
                            <div>X: {state.accelerometer.x.toFixed(4)}</div>
                            <div>Y: {state.accelerometer.y.toFixed(4)}</div>
                            <div>Z: {state.accelerometer.z.toFixed(4)}</div>
                        </div>
                        <div className="sensor-data">
                            <h4>陀螺仪 (rad/s)</h4>
                            <div>X: {state.gyroscope.x.toFixed(4)}</div>
                            <div>Y: {state.gyroscope.y.toFixed(4)}</div>
                            <div>Z: {state.gyroscope.z.toFixed(4)}</div>
                        </div>
                        <div className="sensor-data">
                            <h4>磁力计 (μT)</h4>
                            <div>X: {state.magnetometer.x.toFixed(2)}</div>
                            <div>Y: {state.magnetometer.y.toFixed(2)}</div>
                            <div>Z: {state.magnetometer.z.toFixed(2)}</div>
                        </div>
                    </div>
                </div>

                <div className="section">
                    <h3>设备状态</h3>
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
            </div>
        </div>
    );
}