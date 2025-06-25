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
    orientation: [number, number, number, number];
    accelerometer: [number, number, number];
    gyroscope: [number, number, number];
    magnetometer: [number, number, number];
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
            <div className="top-section">
                <div className="model-view">
                    <Canvas>
                        <Controller3DView state={state} />
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
        </div>
    );
}