import { useState, useEffect } from "react";
import { useTranslation } from 'react-i18next';
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

interface ControllerViewProps {
    isConnected: boolean;
}

export default function ControllerStatus({ isConnected }: ControllerViewProps) {
    const { t } = useTranslation();
    const [state, setState] = useState<ControllerState | null>(null);
    const [battery_level, setBatteryLevel] = useState<number | null>(null);

    // Effect for listeners
    useEffect(() => {
        const setupListeners = async () => {
            const unlistenState = await listen<ControllerState>(
                "controller-state",
                (event) => {
                    setState(event.payload);
                }
            );

            return () => {
                unlistenState();
            };
        };

        const unlistenPromise = setupListeners();

        return () => {
            unlistenPromise.then(unlisten => unlisten && unlisten());
        };
    }, []); // Empty dependency array, runs only once.

    // Effect for polling battery level
    useEffect(() => {
        if (!isConnected) {
            setBatteryLevel(null); // Clear battery level on disconnect
            return;
        }

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
                                        <span>{t('controllerView.buttons.trigger')}</span>
                                    </div>
                                    <div className={`button-indicator ${state.buttons.home ? "active" : ""}`}>
                                        <span>{t('controllerView.buttons.home')}</span>
                                    </div>
                                    <div className={`button-indicator ${state.buttons.back ? "active" : ""}`}>
                                        <span>{t('controllerView.buttons.back')}</span>
                                    </div>
                                    <div className={`button-indicator ${state.buttons.touchpad ? "active" : ""}`}>
                                        <span>{t('controllerView.buttons.touchpad')}</span>
                                    </div>
                                    <div className={`button-indicator ${state.buttons.volume_up ? "active" : ""}`}>
                                        <span>{t('controllerView.buttons.volumeUp')}</span>
                                    </div>
                                    <div className={`button-indicator ${state.buttons.volume_down ? "active" : ""}`}>
                                        <span>{t('controllerView.buttons.volumeDown')}</span>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                    <div className="bottom-section">
                        <div className="device-status">
                            <div className="battery-status">
                                <span>{t('controllerView.battery')}: </span>
                                <div className="battery-bar">
                                    <div
                                        className="battery-level"
                                        style={{ width: `${battery_level}%` }}
                                    ></div>
                                </div>
                                <span>{battery_level || '? '}%</span>
                            </div>
                            <div className="temperature-status">
                                <span>{t('controllerView.temperature')}: </span>
                                <span>{state.temperature.toFixed(1)}°C</span>
                            </div>
                        </div>
                    </div>
                </>
            )}
        </div>
    );
}