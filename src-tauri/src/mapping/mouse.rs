use anyhow::Result;
use log::{info, warn};
use std::thread;
use std::time::{Duration, Instant};
use tauri::AppHandle;
use tokio::sync::mpsc;

use crate::config::keymap_config::KeymapConfig;
use crate::config::mouse_config::MouseConfig;
use crate::core::controller::ControllerState;
use crate::mapping::mouse_mapper::MouseMapper;
enum MouseMapperCommand {
    Update(ControllerState),
    UpdateMouseConfig(MouseConfig),
    UpdateKeymapConfig(KeymapConfig),
}

/// A clonable handle that sends commands to the dedicated MouseMapper thread.
#[derive(Clone)]
pub struct MouseMapperSender {
    pub mouse_config: MouseConfig,
    pub keymap_config: KeymapConfig,
    tx: mpsc::Sender<MouseMapperCommand>,
}

impl MouseMapperSender {
    pub fn new(
        app_handle: &AppHandle,
        mouse_config: MouseConfig,
        keymap_config: KeymapConfig,
    ) -> Self {
        let (tx, mut rx) = mpsc::channel(32);
        let initial_mouse_config = mouse_config.clone();
        let initial_keymap_config = keymap_config.clone();
        let app_handle_clone = app_handle.clone();

        thread::spawn(move || {
            let mut mouse_mapper = MouseMapper::new(
                app_handle_clone,
                initial_mouse_config,
                initial_keymap_config,
            );
            info!("MouseMapper thread with interpolation started.");

            // 定义我们的平滑循环频率，例如 250Hz
            const INTERPOLATION_HZ: u64 = 250;
            let tick_duration = Duration::from_millis(1000 / INTERPOLATION_HZ);
            let mut last_update_time = Instant::now();

            loop {
                // 1. 非阻塞地检查是否有新的控制器数据
                if let Ok(command) = rx.try_recv() {
                    match command {
                        MouseMapperCommand::Update(state) => {
                            // 如果有新数据，就调用 update 来更新【目标位置】
                            mouse_mapper.update(&state);
                            last_update_time = Instant::now();
                        }
                        MouseMapperCommand::UpdateMouseConfig(new_mouse_config) => {
                            info!("Updating Mouse config");
                            mouse_mapper.mouse_config = new_mouse_config;
                        }
                        MouseMapperCommand::UpdateKeymapConfig(new_keymap_config) => {
                            info!("Updating Keymap config");
                            mouse_mapper.keymap_config = new_keymap_config;
                        }
                    }
                }

                // 2. 检查是否超过5秒没有数据更新
                if last_update_time.elapsed() < Duration::from_secs(5) {
                    // 只有最近5秒内有更新时才执行插值计算
                    mouse_mapper.interpolate_tick();
                }

                // 3. 等待一小段时间，以维持固定的循环频率
                thread::sleep(tick_duration);
            }
        });

        Self {
            mouse_config,
            keymap_config,
            tx,
        }
    }

    pub async fn update(&self, state: ControllerState) -> Result<()> {
        self.tx.send(MouseMapperCommand::Update(state)).await?;
        Ok(())
    }

    pub async fn update_mouse_config(&mut self, mouse_config: MouseConfig) {
        self.mouse_config = mouse_config.clone();
        if let Err(e) = self
            .tx
            .send(MouseMapperCommand::UpdateMouseConfig(mouse_config))
            .await
        {
            warn!("Failed to send config update to mouse thread: {}", e);
        }
    }

    pub async fn update_keymap_config(&mut self, keymap_config: KeymapConfig) {
        self.keymap_config = keymap_config.clone();
        if let Err(e) = self
            .tx
            .send(MouseMapperCommand::UpdateKeymapConfig(keymap_config))
            .await
        {
            warn!("Failed to send config update to key mapper thread: {}", e);
        }
    }
}
