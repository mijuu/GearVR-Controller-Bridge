use std::thread;
use std::time::Duration;
use anyhow::Result;
use tokio::sync::mpsc;
use log::{info};

use crate::mapping::mouse_mapper::MouseMapper; 
use crate::core::controller::ControllerState;
enum MouseMapperCommand {
    Update(ControllerState),
}

/// A clonable handle that sends commands to the dedicated MouseMapper thread.
#[derive(Clone)]
pub struct MouseMapperSender {
    tx: mpsc::Sender<MouseMapperCommand>,
}

impl MouseMapperSender {
    pub fn new() -> Self {
        let (tx, mut rx) = mpsc::channel(32);

        thread::spawn(move || {
            let mut mouse_mapper = MouseMapper::new();
            info!("MouseMapper thread with interpolation started.");

            // 定义我们的平滑循环频率，例如 250Hz
            const INTERPOLATION_HZ: u64 = 250;
            let tick_duration = Duration::from_millis(1000 / INTERPOLATION_HZ);

            loop {
                // 1. 非阻塞地检查是否有新的控制器数据
                if let Ok(command) = rx.try_recv() {
                    match command {
                        MouseMapperCommand::Update(state) => {
                            // 如果有新数据，就调用 update 来更新【目标位置】
                            mouse_mapper.update(&state);
                        }
                    }
                }

                // 2. 无论有没有新数据，每一帧都执行平滑插值计算和移动
                mouse_mapper.interpolate_tick();

                // 3. 等待一小段时间，以维持固定的循环频率
                thread::sleep(tick_duration);
            }
        });

        Self { tx }
    }

    pub async fn update(&self, state: ControllerState) -> Result<()> {
        self.tx.send(MouseMapperCommand::Update(state)).await?;
        Ok(())
    }
}

impl Default for MouseMapperSender {
    fn default() -> Self {
        Self::new()
    }
}