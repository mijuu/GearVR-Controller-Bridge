//! Mouse and keyboard mapping for GearVR controller
//! This module maps controller inputs to mouse and keyboard actions using the enigo library.

use enigo::{Enigo, KeyboardControllable, MouseButton, MouseControllable};
use nalgebra::UnitQuaternion;
use serde::{Deserialize, Serialize};

use crate::core::controller::{ButtonState, ControllerState, TouchpadState};

/// Configuration for button mappings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonMapping {
    /// Trigger button mapping
    pub trigger: Option<String>,
    /// Home button mapping
    pub home: Option<String>,
    /// Back button mapping
    pub back: Option<String>,
    /// Volume up button mapping
    pub volume_up: Option<String>,
    /// Volume down button mapping
    pub volume_down: Option<String>,
    /// Touchpad click mapping
    pub touchpad: Option<String>,
}

impl Default for ButtonMapping {
    fn default() -> Self {
        ButtonMapping {
            trigger: Some("left".to_string()),   // Left mouse button by default
            home: Some("esc".to_string()),       // Escape key by default
            back: Some("backspace".to_string()), // Backspace key by default
            volume_up: Some("volume_up".to_string()), // Volume up
            volume_down: Some("volume_down".to_string()), // Volume down
            touchpad: Some("right".to_string()), // Right mouse button by default
        }
    }
}

/// Mouse movement mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MouseMode {
    /// Use controller orientation to control mouse movement
    Orientation,
    /// Use touchpad to control mouse movement (like laptop touchpad)
    Touchpad,
}

/// Mouse mapper configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseMapperConfig {
    /// Mouse movement mode
    pub mode: MouseMode,
    /// Button mappings
    pub button_mapping: ButtonMapping,
    /// Mouse sensitivity for orientation mode
    pub orientation_sensitivity: f32,
    /// Mouse sensitivity for touchpad mode
    pub touchpad_sensitivity: f32,
    /// Acceleration factor for touchpad mode. 0.0 means no acceleration.
    pub touchpad_acceleration: f32,
    /// The speed threshold to activate acceleration. Below this, movement is linear (precise).
    /// The unit is abstract, related to (distance_squared / time_delta).
    pub touchpad_acceleration_threshold: f32,
}

impl Default for MouseMapperConfig {
    fn default() -> Self {
        MouseMapperConfig {
            mode: MouseMode::Touchpad,
            button_mapping: ButtonMapping::default(),
            orientation_sensitivity: 10.0,
            touchpad_sensitivity: 500.0,
            touchpad_acceleration: 1.2,
            touchpad_acceleration_threshold: 0.0002,
        }
    }
}

/// Maps controller inputs to mouse and keyboard actions
pub struct MouseMapper {
    /// Enigo instance for input simulation
    enigo: Enigo,
    /// Current configuration
    config: MouseMapperConfig,
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
}

impl MouseMapper {
    /// Creates a new mouse mapper with default configuration
    pub fn new() -> Self {
        let enigo = Enigo::new();
        let (x, y) = enigo.mouse_location();
        Self {
            enigo,
            config: MouseMapperConfig::default(),
            last_state: None,
            remainder_x: 0.0,
            remainder_y: 0.0,
            target_screen_x: x,
            target_screen_y: y,
            is_touchpad_active: false,
        }
    }

    /// Creates a new mouse mapper with custom configuration
    pub fn with_config(config: MouseMapperConfig) -> Self {
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
        }
    }

    /// Updates the mouse mapper with new controller state
    pub fn update(&mut self, state: &ControllerState) {
        // 步骤 1: 检查 `last_state` 是否存在。如果存在，克隆出所需的数据。
        let last_state_data = self
            .last_state
            .as_ref()
            .map(|last| (last.buttons.clone(), last.touchpad.clone(), last.timestamp));

        // 步骤 2: 现在 `self` 没有被任何借用持有，我们可以自由地调用可变方法了。
        if let Some((last_buttons, last_touchpad, last_timestamp)) = last_state_data {
            // --- 按钮处理 ---
            self.handle_buttons(&state.buttons, &last_buttons);

            // --- 移动处理 ---
            match self.config.mode {
                MouseMode::Orientation => {
                    self.handle_orientation_movement(&state.orientation);
                }
                MouseMode::Touchpad => {
                    let delta_t = (state.timestamp - last_timestamp) as f32;
                    self.handle_touchpad_movement(&state.touchpad, &last_touchpad, delta_t);
                }
            }
        } else {
            // 只处理按钮的初次按下，没有弹起。
            let default_buttons = ButtonState::default();
            self.handle_buttons(&state.buttons, &default_buttons);
        }

        // 步骤 3: 在所有操作的最后，更新 `last_state` 以供下一帧使用。
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

    /// Handles mouse movement in orientation mode
    fn handle_orientation_movement(&mut self, orientation: &UnitQuaternion<f64>) {
        // Get current mouse position
        let (x, y) = self.enigo.mouse_location();

        // Convert quaternion to pitch and yaw angles
        let (pitch, yaw, _) = orientation.euler_angles();

        // Calculate mouse movement based on orientation
        let dx = (yaw.to_degrees() * self.config.orientation_sensitivity as f64) as i32;
        let dy = (pitch.to_degrees() * self.config.orientation_sensitivity as f64) as i32;

        // Move mouse relative to current position
        self.enigo.mouse_move_relative(dx, -dy); // Invert y-axis for natural movement
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
        if !self.is_touchpad_active {
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

    /// Changes the mouse movement mode
    pub fn set_mode(&mut self, mode: MouseMode) {
        self.config.mode = mode;
    }

    /// Updates the button mapping configuration
    pub fn update_button_mapping(&mut self, mapping: ButtonMapping) {
        self.config.button_mapping = mapping;
    }
}

impl Default for MouseMapper {
    fn default() -> Self {
        Self::new()
    }
}
