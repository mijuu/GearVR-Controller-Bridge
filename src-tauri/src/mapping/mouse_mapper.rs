//! Mouse and keyboard mapping for GearVR controller
//! This module maps controller inputs to mouse and keyboard actions using the enigo library.

use enigo::{Enigo, KeyboardControllable, MouseButton, MouseControllable};
use nalgebra::UnitQuaternion;

use crate::core::controller::{ButtonState, ControllerState, TouchpadState};
use crate::config::mouse_mapper_config::{MouseMode, MouseMapperConfig};

/// Maps controller inputs to mouse and keyboard actions
pub struct MouseMapper {
    /// Enigo instance for input simulation
    enigo: Enigo,
    /// Current configuration
    pub config: MouseMapperConfig,
    /// Last controller state
    last_state: Option<ControllerState>,
    /// Accumulators for sub-pixel movements
    remainder_x: f32,
    remainder_y: f32,
    /// The screen coordinates where the mouse should be heading.
    target_screen_x: i32,
    target_screen_y: i32,
    /// A flag to indicate if the touchpad is currently being used.
    is_touchpad_active: bool,
    /// A flag to indicate if the air mouse is currently being used.
    is_air_mouse_active: bool,
}

impl MouseMapper {
    /// Creates a new mouse mapper with default configuration
    pub fn new(config: MouseMapperConfig) -> Self {
        let enigo = Enigo::new();
        let (x, y) = enigo.mouse_location();
        Self {
            enigo,
            config,
            last_state: None,
            remainder_x: 0.0,
            remainder_y: 0.0,
            target_screen_x: x,
            target_screen_y: y,
            is_touchpad_active: false,
            is_air_mouse_active: false,
        }
    }

    /// Updates the mouse mapper with new controller state
    pub fn update(&mut self, state: &ControllerState) {
        // 提取上一次的状态数据
        let last_state_data = self
            .last_state
            .as_ref()
            .map(|last| (last.buttons.clone(), last.touchpad.clone(), last.timestamp));

        if let Some((last_buttons, last_touchpad, last_timestamp)) = last_state_data {
            // --- 步骤 1: 按钮处理（所有模式通用） ---
            self.handle_buttons(&state.buttons, &last_buttons);

            // --- 步骤 2: 使用 match 彻底分离不同模式的移动逻辑 ---
            match self.config.mode {
                MouseMode::AirMouse => {
                    let delta_t_ms = state.timestamp - last_timestamp;
                    if delta_t_ms > 0 {
                        let delta_t_s = delta_t_ms as f32 / 1000.0;

                        let last_orientation = self.last_state.as_ref().unwrap().orientation;
                        let delta_orientation = last_orientation.inverse() * state.orientation;
                        let rotation_angle_deg = delta_orientation.angle().to_degrees() as f32;
                        let rotational_speed_dps = rotation_angle_deg / delta_t_s;

                        // 更新激活状态
                        self.is_air_mouse_active =
                            rotational_speed_dps > self.config.air_mouse_activation_threshold;
                    }

                    // 只有在激活时才计算目标点
                    if self.is_air_mouse_active {
                        self.handle_air_mouse_movement(&state.orientation);
                    }

                    // 确保在 AirMouse 模式下，touchpad 状态被重置
                    self.is_touchpad_active = false;
                }
                MouseMode::Touchpad => {
                    let delta_t = (state.timestamp - last_timestamp) as f32;
                    self.handle_touchpad_movement(&state.touchpad, &last_touchpad, delta_t);

                    // 确保在 Touchpad 模式下，air_mouse 状态被重置
                    self.is_air_mouse_active = false;
                }
            }
        } else {
            // 处理第一帧的按钮按下
            let default_buttons = ButtonState::default();
            self.handle_buttons(&state.buttons, &default_buttons);
        }

        // --- 步骤 3: 更新 last_state (所有模式通用) ---
        self.last_state = Some(state.clone());
    }

    /// Handles button state changes by comparing the current state to the last one.
    fn handle_buttons(&mut self, current: &ButtonState, last: &ButtonState) {
        let mapping = self.config.button_mapping.clone();

        // Helper closure to process a single button's state change
        let mut process_change = |is_pressed: bool, was_pressed: bool, key_map: &Option<String>| {
            if let Some(key) = key_map {
                if is_pressed && !was_pressed {
                    // State changed from UP to DOWN: Press the key
                    self.press_key(key);
                } else if !is_pressed && was_pressed {
                    // State changed from DOWN to UP: Release the key
                    self.release_key(key);
                }
            }
        };

        // Process each button
        process_change(current.trigger, last.trigger, &mapping.trigger);
        process_change(current.home, last.home, &mapping.home);
        process_change(current.back, last.back, &mapping.back);
        process_change(current.volume_up, last.volume_up, &mapping.volume_up);
        process_change(current.volume_down, last.volume_down, &mapping.volume_down);
        process_change(current.touchpad, last.touchpad, &mapping.touchpad);
    }

    /// Presses a key or mouse button based on string identifier
    fn press_key(&mut self, key: &str) {
        eprintln!("Pressing key: {}", key);
        match key.to_lowercase().as_str() {
            // 鼠标按键
            "left" => self.enigo.mouse_down(MouseButton::Left),
            "right" => self.enigo.mouse_down(MouseButton::Right),
            "middle" => self.enigo.mouse_down(MouseButton::Middle),

            // 特殊功能键
            "esc" | "escape" => self.enigo.key_click(enigo::Key::Escape),
            "backspace" => self.enigo.key_click(enigo::Key::Backspace),

            // 多媒体键 (注意：这些键的可用性取决于操作系统和 enigo 的支持)
            "volume_up" => self.enigo.key_click(enigo::Key::VolumeUp),
            "volume_down" => self.enigo.key_click(enigo::Key::VolumeDown),

            // 其他常用键的示例
            "enter" => self.enigo.key_click(enigo::Key::Return), // 或者 Key::Enter
            "tab" => self.enigo.key_click(enigo::Key::Tab),
            "space" => self.enigo.key_click(enigo::Key::Space),
            "home" => self.enigo.key_click(enigo::Key::Home),
            "end" => self.enigo.key_click(enigo::Key::End),
            "pageup" => self.enigo.key_click(enigo::Key::PageUp),
            "pagedown" => self.enigo.key_click(enigo::Key::PageDown),
            "shift" => self.enigo.key_click(enigo::Key::Shift),
            "ctrl" | "control" => self.enigo.key_click(enigo::Key::Control),
            "alt" => self.enigo.key_click(enigo::Key::Alt),
            // F1 到 F12
            "f1" => self.enigo.key_click(enigo::Key::F1),
            "f2" => self.enigo.key_click(enigo::Key::F2),
            // ...以此类推...
            "f12" => self.enigo.key_click(enigo::Key::F12),

            // 默认情况：处理单个字符
            // 只有当以上所有情况都不匹配时，才认为它是一个普通字符
            single_char_key => {
                if let Some(c) = single_char_key.chars().next() {
                    self.enigo.key_click(enigo::Key::Layout(c));
                }
            }
        }
    }

    /// Releases a key or mouse button based on string identifier
    fn release_key(&mut self, key: &str) {
        // We only need to handle the release for mouse buttons that were pressed with mouse_down.
        // Keyboard keys handled by key_click already include a release.
        match key.to_lowercase().as_str() {
            "left" => self.enigo.mouse_up(MouseButton::Left),
            "right" => self.enigo.mouse_up(MouseButton::Right),
            "middle" => self.enigo.mouse_up(MouseButton::Middle),
            // For all other keys, do nothing on release.
            _ => {}
        }
    }

    /// Handles mouse movement in air mouse mode using absolute position mapping.
    fn handle_air_mouse_movement(&mut self, orientation: &UnitQuaternion<f64>) {
        // --- 步骤 1: 将原始四元数变换到显示坐标系 ---
        let transformed_quat =
            nalgebra::Quaternion::new(orientation.w, orientation.j, orientation.i, -orientation.k);
        let transformed_orientation = UnitQuaternion::new_normalize(transformed_quat);
        // --- 步骤 2: 从【变换后】的四元数中提取欧拉角 (此部分逻辑不变) ---
        let (_roll, pitch, yaw) = transformed_orientation.euler_angles();
        let horizontal_deg = yaw.to_degrees() as f32;
        let vertical_deg = pitch.to_degrees() as f32;

        // --- 步骤 3: 将正确的角度映射到屏幕坐标 (此部分逻辑不变) ---
        let (screen_width, screen_height) = self.enigo.main_display_size();

        // 【方向微调】根据需要对角度取反
        let x_ratio = (horizontal_deg / self.config.air_mouse_fov) + 0.5;
        let aspect_ratio = screen_height as f32 / screen_width as f32;
        let vertical_fov = self.config.air_mouse_fov * aspect_ratio;
        let y_ratio = (-vertical_deg / vertical_fov) + 0.5;

        let target_x = (x_ratio * screen_width as f32).round() as i32;
        let target_y = (y_ratio * screen_height as f32).round() as i32;

        self.target_screen_x = target_x.clamp(0, screen_width as i32 - 1);
        self.target_screen_y = target_y.clamp(0, screen_height as i32 - 1);
    }

    /// Handles mouse movement in touchpad mode with relative tracking and acceleration.
    /// This function is now stateless and relies only on its inputs.
    fn handle_touchpad_movement(
        &mut self,
        current_touchpad: &TouchpadState,
        last_touchpad: &TouchpadState,
        delta_t: f32,
    ) {
        if current_touchpad.touched {
            // 手指正在触摸，标记为活跃状态
            self.is_touchpad_active = true;
            // 只有当上一帧也是触摸状态时，才计算移动
            if last_touchpad.touched {
                let delta_x = current_touchpad.x - last_touchpad.x;
                let delta_y = current_touchpad.y - last_touchpad.y;

                if delta_t <= 0.0 {
                    return;
                }

                let speed_sq = (delta_x.powi(2) + delta_y.powi(2)) / delta_t;
                let effective_speed_sq =
                    (speed_sq - self.config.touchpad_acceleration_threshold).max(0.0);
                let acceleration_multiplier =
                    1.0 + (effective_speed_sq * 500.0 * self.config.touchpad_acceleration);
                let base_dx = delta_x * self.config.touchpad_sensitivity;
                let base_dy = delta_y * self.config.touchpad_sensitivity;

                // 1. 计算本帧期望的、包含小数的浮点移动量
                let desired_dx_float = base_dx * acceleration_multiplier;
                let desired_dy_float = base_dy * acceleration_multiplier;

                // 2. 将期望移动量与上一帧存留的“零钱”（remainder）相加
                let total_dx_float = desired_dx_float + self.remainder_x;
                let total_dy_float = desired_dy_float + self.remainder_y;

                // 3. 实际要移动的整数像素是总和的整数部分
                let final_dx = total_dx_float.trunc() as i32;
                let final_dy = total_dy_float.trunc() as i32;

                // 4. 将总和剩下的小数部分存回“零钱罐”，供下一帧使用
                self.remainder_x = total_dx_float.fract();
                self.remainder_y = total_dy_float.fract();

                if final_dx != 0 || final_dy != 0 {
                    let target_x = self.target_screen_x + final_dx;
                    let target_y = self.target_screen_y + final_dy;

                    let (screen_width, screen_height) = self.enigo.main_display_size();
                    self.target_screen_x = target_x.clamp(0, screen_width as i32 - 1);
                    self.target_screen_y = target_y.clamp(0, screen_height as i32 - 1);
                }
            }
        } else {
            // 手指已离开，标记为非活跃状态
            self.is_touchpad_active = false;
        }
    }

    /// Performs one step of interpolation towards the target position.
    /// This should be called at a high, fixed frequency.
    pub fn interpolate_tick(&mut self) {
        // 检查触摸板是否处于非活跃状态
        if !self.is_touchpad_active && !self.is_air_mouse_active {
            // 1. 获取鼠标当前在操作系统中的真实位置
            let (current_x, current_y) = self.enigo.mouse_location();
            self.target_screen_x = current_x;
            self.target_screen_y = current_y;

            // 同时清空亚像素累加器
            self.remainder_x = 0.0;
            self.remainder_y = 0.0;

            return;
        }
        let (current_x, current_y) = self.enigo.mouse_location();
        // 2. 计算当前位置到目标位置的距离
        let dx = self.target_screen_x - current_x;
        let dy = self.target_screen_y - current_y;

        // 如果距离已经很小（小于1像素），就直接跳到目标位置，避免抖动
        if dx.abs() < 1 && dy.abs() < 1 {
            // 只有在目标与当前位置不同时才移动，防止不必要的系统调用
            if current_x != self.target_screen_x || current_y != self.target_screen_y {
                self.enigo
                    .mouse_move_to(self.target_screen_x, self.target_screen_y);
            }
            return;
        }

        // 3. 线性插值（Lerp）：每次移动一小部分距离（例如30%）
        // 这个 "0.3" 是平滑因子，可以按需调整
        const SMOOTHING_FACTOR: f32 = 0.3;
        let new_x = current_x + (dx as f32 * SMOOTHING_FACTOR) as i32;
        let new_y = current_y + (dy as f32 * SMOOTHING_FACTOR) as i32;

        // 4. 执行这一小步平滑移动
        self.enigo.mouse_move_to(new_x, new_y);
    }
}

impl Default for MouseMapper {
    fn default() -> Self {
        Self::new(MouseMapperConfig::default())
    }
}
