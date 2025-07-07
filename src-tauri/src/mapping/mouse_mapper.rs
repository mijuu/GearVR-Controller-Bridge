//! Mouse and keyboard mapping for GearVR controller
//! This module maps controller inputs to mouse and keyboard actions using the enigo library.

use enigo::{Enigo, KeyboardControllable, MouseButton, MouseControllable};
use nalgebra::UnitQuaternion;

use crate::config::mouse_mapper_config::{MouseMapperConfig, MouseMode};
use crate::core::controller::{ButtonState, ControllerState, TouchpadState};

/// Maps controller inputs to mouse and keyboard actions
pub struct MouseMapper {
    /// Enigo instance for input simulation
    enigo: Enigo,
    /// Current configuration
    pub config: MouseMapperConfig,
    /// Last controller state
    last_state: Option<ControllerState>,
    /// Accumulators for sub-pixel movements from the touchpad.
    remainder_x: f32,
    remainder_y: f32,
    /// The screen coordinates where the mouse should be heading.
    target_screen_x: i32,
    target_screen_y: i32,
    /// A flag to indicate if precision mode is currently active (i.e., touchpad is touched).
    is_precision_mode_active: bool,
    /// A flag to indicate if the air mouse movement is currently active.
    is_air_mouse_active: bool,
    /// Timestamp of the last home button press, for double-click detection.
    home_button_last_press_time: Option<u64>,
    // --- Fields for seamless precision mode transition ---
    /// The controller's yaw when precision mode was activated.
    precision_mode_center_yaw: f32,
    /// The controller's pitch when precision mode was activated.
    precision_mode_center_pitch: f32,
    /// The mouse's screen X coordinate when precision mode was activated.
    precision_mode_start_x: i32,
    /// The mouse's screen Y coordinate when precision mode was activated.
    precision_mode_start_y: i32,
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
            is_precision_mode_active: false,
            is_air_mouse_active: false,
            home_button_last_press_time: None,
            precision_mode_center_yaw: 0.0,
            precision_mode_center_pitch: 0.0,
            precision_mode_start_x: 0,
            precision_mode_start_y: 0,
        }
    }

    /// Updates the mouse mapper with new controller state
    pub fn update(&mut self, state: &ControllerState) {
        let last_state_data = self
            .last_state
            .as_ref()
            .map(|last| (last.buttons.clone(), last.touchpad.clone(), last.timestamp));

        if let Some((last_buttons, last_touchpad, last_timestamp)) = last_state_data {
            // --- Home button double-click detection to toggle mouse mode ---
            if state.buttons.home && !last_buttons.home {
                const DOUBLE_CLICK_WINDOW_MS: u64 = 300;
                let now = state.timestamp;

                if let Some(last_press_time) = self.home_button_last_press_time {
                    if now.saturating_sub(last_press_time) < DOUBLE_CLICK_WINDOW_MS {
                        self.config.mode = match self.config.mode {
                            MouseMode::AirMouse => MouseMode::Touchpad,
                            MouseMode::Touchpad => MouseMode::AirMouse,
                        };
                        self.home_button_last_press_time = None; // Reset timer
                    } else {
                        self.home_button_last_press_time = Some(now);
                    }
                } else {
                    self.home_button_last_press_time = Some(now);
                }
            }

            // --- Step 1: Handle button presses (common to all modes) ---
            self.handle_buttons(&state.buttons, &last_buttons);

            // --- Step 2: Handle movement based on the current mode ---
            let delta_t = (state.timestamp - last_timestamp) as f32;

            match self.config.mode {
                MouseMode::AirMouse => {
                    // --- Air Mouse Mode Logic ---

                    // Store the previous state of precision mode to detect transitions.
                    let was_precision_mode_active = self.is_precision_mode_active;

                    // Precision mode is active only when the touchpad is touched.
                    self.is_precision_mode_active = state.touchpad.touched;

                    // Determine if we are *entering* precision mode in this frame.
                    let is_entering_precision_mode =
                        self.is_precision_mode_active && !was_precision_mode_active;

                    let delta_t_ms = state.timestamp - last_timestamp;
                    if delta_t_ms > 0 {
                        let delta_t_s = delta_t_ms as f32 / 1000.0;
                        let last_orientation = self.last_state.as_ref().unwrap().orientation;
                        let delta_orientation = last_orientation.inverse() * state.orientation;
                        let rotation_angle_deg = delta_orientation.angle().to_degrees() as f32;
                        // Calculate rotational speed to determine if air mouse is active.
                        let rotational_speed_dps = rotation_angle_deg / delta_t_s;

                        self.is_air_mouse_active =
                            rotational_speed_dps > self.config.air_mouse_activation_threshold;
                    }

                    // Handle touchpad movement, which adds to the target position.
                    // self.handle_touchpad_movement(&state.touchpad, &last_touchpad, delta_t);

                    // Only calculate air mouse movement if it's active or if we are in precision mode.
                    if self.is_air_mouse_active || self.is_precision_mode_active {
                        self.handle_air_mouse_movement(
                            &state.orientation,
                            self.is_precision_mode_active,
                            is_entering_precision_mode,
                        );
                    }
                }
                MouseMode::Touchpad => {
                    // --- Touchpad-Only Mode Logic ---
                    // In Touchpad mode, precision mode is implicitly active if touchpad is touched.
                    self.is_precision_mode_active = state.touchpad.touched; // Set based on current touchpad state
                    self.handle_touchpad_movement(&state.touchpad, &last_touchpad, delta_t);
                    self.is_air_mouse_active = false;
                }
            }
        } else {
            // Handle button presses for the very first frame.
            let default_buttons = ButtonState::default();
            self.handle_buttons(&state.buttons, &default_buttons);
        }

        // --- Step 3: Update the last state for the next frame ---
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

    /// Handles mouse movement in air mouse mode.
    /// Switches between absolute positioning and relative (precision) positioning.
    fn handle_air_mouse_movement(
        &mut self,
        orientation: &UnitQuaternion<f64>,
        is_precision_mode_active: bool,
        is_entering_precision_mode: bool,
    ) {
        // --- Step 1: Transform the raw quaternion to the display coordinate system ---
        let transformed_quat =
            nalgebra::Quaternion::new(orientation.w, orientation.j, orientation.i, -orientation.k);
        let transformed_orientation = UnitQuaternion::new_normalize(transformed_quat);

        // --- Step 2: Extract Euler angles from the transformed quaternion ---
        let (_roll, pitch, yaw) = transformed_orientation.euler_angles();
        let horizontal_deg = yaw.to_degrees() as f32;
        let vertical_deg = pitch.to_degrees() as f32;

        // --- Step 3: On entering precision mode, capture the initial state for seamless transition ---
        if is_entering_precision_mode {
            // Set the current controller orientation as the center point for relative calculations.
            self.precision_mode_center_yaw = horizontal_deg;
            self.precision_mode_center_pitch = vertical_deg;

            // Set the current mouse position as the starting point for relative movement.
            let (x, y) = self.enigo.mouse_location();
            self.precision_mode_start_x = x;
            self.precision_mode_start_y = y;
        }

        let (screen_width, screen_height) = self.enigo.main_display_size();

        if is_precision_mode_active {
            // --- Precision Mode: Relative movement based on the initial state ---

            // 1. Calculate the angular deviation from the center point.
            let delta_yaw = horizontal_deg - self.precision_mode_center_yaw;
            let delta_pitch = vertical_deg - self.precision_mode_center_pitch;

            // 2. Define sensitivity for precision mode. A larger FOV means slower movement.
            const PRECISION_MODE_SENSITIVITY_FACTOR: f32 = 10.0;
            let effective_fov = self.config.air_mouse_fov * PRECISION_MODE_SENSITIVITY_FACTOR;
            let aspect_ratio = screen_height as f32 / screen_width as f32;
            let vertical_fov = effective_fov * aspect_ratio;

            // 3. Convert angular deviation to pixel offset.
            let offset_x = (delta_yaw / effective_fov) * screen_width as f32;
            let offset_y = (-delta_pitch / vertical_fov) * screen_height as f32;

            // 4. Calculate the final target position: start point + offset.
            let target_x = self.precision_mode_start_x + offset_x.round() as i32;
            let target_y = self.precision_mode_start_y + offset_y.round() as i32;

            self.target_screen_x = target_x.clamp(0, screen_width as i32 - 1);
            self.target_screen_y = target_y.clamp(0, screen_height as i32 - 1);
        } else {
            // --- Normal Mode: Absolute position mapping ---
            let x_ratio = (horizontal_deg / self.config.air_mouse_fov) + 0.5;
            let aspect_ratio = screen_height as f32 / screen_width as f32;
            let vertical_fov = self.config.air_mouse_fov * aspect_ratio;
            let y_ratio = (-vertical_deg / vertical_fov) + 0.5;

            let target_x = (x_ratio * screen_width as f32).round() as i32;
            let target_y = (y_ratio * screen_height as f32).round() as i32;

            self.target_screen_x = target_x.clamp(0, screen_width as i32 - 1);
            self.target_screen_y = target_y.clamp(0, screen_height as i32 - 1);
        }
    }

    /// Handles mouse movement from the touchpad with relative tracking and acceleration.
    /// This function now only calculates the relative movement and updates the target position.
    fn handle_touchpad_movement(
        &mut self,
        current_touchpad: &TouchpadState,
        last_touchpad: &TouchpadState,
        delta_t: f32,
    ) {
        // Only calculate movement if the finger is touched now and was also touched last frame.
        if current_touchpad.touched && last_touchpad.touched {
            let delta_x = current_touchpad.x - last_touchpad.x;
            let delta_y = current_touchpad.y - last_touchpad.y;

            if delta_t <= 0.0 {
                return;
            }

            // Acceleration logic
            let speed_sq = (delta_x.powi(2) + delta_y.powi(2)) / delta_t;
            let effective_speed_sq =
                (speed_sq - self.config.touchpad_acceleration_threshold).max(0.0);
            let acceleration_multiplier =
                1.0 + (effective_speed_sq * 500.0 * self.config.touchpad_acceleration);
            let base_dx = delta_x * self.config.touchpad_sensitivity;
            let base_dy = delta_y * self.config.touchpad_sensitivity;

            // Sub-pixel movement logic
            let desired_dx_float = base_dx * acceleration_multiplier;
            let desired_dy_float = base_dy * acceleration_multiplier;

            let total_dx_float = desired_dx_float + self.remainder_x;
            let total_dy_float = desired_dy_float + self.remainder_y;

            let final_dx = total_dx_float.trunc() as i32;
            let final_dy = total_dy_float.trunc() as i32;

            self.remainder_x = total_dx_float.fract();
            self.remainder_y = total_dy_float.fract();

            // Apply movement to the target position
            if final_dx != 0 || final_dy != 0 {
                let target_x = self.target_screen_x + final_dx;
                let target_y = self.target_screen_y + final_dy;

                let (screen_width, screen_height) = self.enigo.main_display_size();
                self.target_screen_x = target_x.clamp(0, screen_width as i32 - 1);
                self.target_screen_y = target_y.clamp(0, screen_height as i32 - 1);
            }
        }
    }

    /// Performs one step of interpolation towards the target position.
    /// This should be called at a high, fixed frequency.
    pub fn interpolate_tick(&mut self) {
        // If no input is active, sync the target position with the actual mouse position.
        if !self.is_precision_mode_active && !self.is_air_mouse_active {
            let (current_x, current_y) = self.enigo.mouse_location();
            self.target_screen_x = current_x;
            self.target_screen_y = current_y;

            // Reset sub-pixel accumulators
            self.remainder_x = 0.0;
            self.remainder_y = 0.0;

            return;
        }

        let (current_x, current_y) = self.enigo.mouse_location();
        let dx = self.target_screen_x - current_x;
        let dy = self.target_screen_y - current_y;

        // If the distance is negligible, snap to the target to prevent jitter.
        if dx.abs() < 1 && dy.abs() < 1 {
            if current_x != self.target_screen_x || current_y != self.target_screen_y {
                self.enigo
                    .mouse_move_to(self.target_screen_x, self.target_screen_y);
            }
            return;
        }

        // Smoothly interpolate towards the target position.
        const SMOOTHING_FACTOR: f32 = 0.3;
        let new_x = current_x + (dx as f32 * SMOOTHING_FACTOR) as i32;
        let new_y = current_y + (dy as f32 * SMOOTHING_FACTOR) as i32;

        self.enigo.mouse_move_to(new_x, new_y);
    }
}

impl Default for MouseMapper {
    fn default() -> Self {
        Self::new(MouseMapperConfig::default())
    }
}

