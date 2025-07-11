//! Mouse and keyboard mapping for GearVR controller
//! This module maps controller inputs to mouse and keyboard actions using the enigo library.

use anyhow::{Ok, Result};
use enigo::{
    Button, Coordinate, Direction,
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Mouse, Settings,
};
use nalgebra::UnitQuaternion;
use tauri::AppHandle; // Import AppHandle

use crate::config::keymap_config::KeymapConfig;
use crate::config::mouse_config::{MouseConfig, MouseMode};
use crate::core::controller::{ButtonState, ControllerState, TouchpadState};

/// Maps controller inputs to mouse and keyboard actions
pub struct MouseMapper {
    /// Enigo instance for input simulation
    enigo: Enigo,
    /// The Tauri app handle to run commands on the main thread
    app_handle: AppHandle,
    /// Current mouse configuration
    pub mouse_config: MouseConfig,
    /// Current keymap configuration
    pub keymap_config: KeymapConfig,
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
    pub fn new(app_handle: AppHandle, mouse_config: MouseConfig, keymap_config: KeymapConfig) -> Self {
        let enigo = Enigo::new(&Settings::default()).unwrap();
        let (x, y) = enigo.location().unwrap();
        Self {
            enigo,
            app_handle,
            mouse_config,
            keymap_config,
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
                        self.mouse_config.mode = match self.mouse_config.mode {
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

            match self.mouse_config.mode {
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
                            rotational_speed_dps > self.mouse_config.air_mouse_activation_threshold;
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
        let mapping = self.keymap_config.clone();

        // Helper closure to process a single button's state change
        let mut process_change = |is_pressed: bool, was_pressed: bool, key_map: &Option<String>| {
            if let Some(key) = key_map {
                if is_pressed && !was_pressed {
                    // State changed from UP to DOWN: Press the key
                    if let Err(e) = self.press_key(key) {
                        eprintln!("Failed to press key '{}': {:?}", key, e);
                    }
                } else if !is_pressed && was_pressed {
                    // State changed from DOWN to UP: Release the key
                    if let Err(e) = self.release_key(key) {
                        eprintln!("Failed to release key '{}': {:?}", key, e);
                    }
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

    /// Determines if a key string is a modifier key.
    fn is_modifier(key: &str) -> bool {
        matches!(
            key.to_lowercase().as_str(),
            "ctrl" | "control" | "shift" | "alt" | "meta" | "win" | "command"
        )
    }

    /// Converts a key string to an `enigo::Key`.
    fn string_to_key(key: &str) -> Option<Key> {
        match key.to_lowercase().as_str() {
            "esc" | "escape" => Some(Key::Escape),
            "backspace" => Some(Key::Backspace),
            "volume up" => Some(Key::VolumeUp),
            "volume down" => Some(Key::VolumeDown),
            "enter" => Some(Key::Return),
            "tab" => Some(Key::Tab),
            "space" => Some(Key::Space),
            "home" => Some(Key::Home),
            "end" => Some(Key::End),
            "pageup" => Some(Key::PageUp),
            "pagedown" => Some(Key::PageDown),
            "shift" => Some(Key::Shift),
            "ctrl" | "control" => Some(Key::Control),
            "alt" => Some(Key::Alt),
            "meta" | "win" | "command" => Some(Key::Meta),
            "f1" => Some(Key::F1),
            "f2" => Some(Key::F2),
            "f3" => Some(Key::F3),
            "f4" => Some(Key::F4),
            "f5" => Some(Key::F5),
            "f6" => Some(Key::F6),
            "f7" => Some(Key::F7),
            "f8" => Some(Key::F8),
            "f9" => Some(Key::F9),
            "f10" => Some(Key::F10),
            "f11" => Some(Key::F11),
            "f12" => Some(Key::F12),
            // Prevent multi-character strings like "left" from being treated as Unicode
            single_char if single_char.chars().count() == 1 => {
                single_char.chars().next().map(Key::Unicode)
            }
            // It's not a recognized key (e.g., it's a mouse button or an unknown key)
            _ => None,
        }
    }

    /// Presses a key or mouse button based on string identifier.
    fn press_key(&mut self, key_str: &str) -> Result<()> {
        // Check if any part of the key string requires the main thread.
        let needs_main_thread = key_str
            .split('+')
            .any(|part| matches!(Self::string_to_key(part.trim()), Some(Key::Unicode(_))));

        if needs_main_thread {
            // If so, execute the entire operation on the main thread.
            let app_handle = self.app_handle.clone();
            let key_string = key_str.to_string();
            app_handle.run_on_main_thread(move || {
                let mut enigo = Enigo::new(&Settings::default()).unwrap();
                Self::execute_key_sequence(&mut enigo, &key_string, Press).unwrap();
            })?;
        } else {
            // Otherwise, execute on the current thread.
            Self::execute_key_sequence(&mut self.enigo, key_str, Press)?;
        }
        Ok(())
    }

    /// Releases a key or mouse button based on string identifier.
    fn release_key(&mut self, key_str: &str) -> Result<()> {
        let needs_main_thread = key_str
            .split('+')
            .any(|part| matches!(Self::string_to_key(part.trim()), Some(Key::Unicode(_))));

        if needs_main_thread {
            let app_handle = self.app_handle.clone();
            let key_string = key_str.to_string();
            app_handle.run_on_main_thread(move || {
                let mut enigo = Enigo::new(&Settings::default()).unwrap();
                Self::execute_key_sequence(&mut enigo, &key_string, Release).unwrap();
            })?;
        } else {
            Self::execute_key_sequence(&mut self.enigo, key_str, Release)?;
        }
        Ok(())
    }

    /// Helper function to execute the actual key sequence on a given enigo instance.
    fn execute_key_sequence(enigo: &mut Enigo, key_str: &str, direction: Direction) -> Result<()> {
        let parts: Vec<&str> = key_str.split('+').map(|k| k.trim()).collect();

        // Separate parts into modifiers, regular keys, and mouse buttons
        let mut modifier_keys = Vec::new();
        let mut action_keys = Vec::new();
        let mut mouse_buttons = Vec::new();

        for part in parts {
            let lower_part = part.to_lowercase();
            if Self::is_modifier(&lower_part) {
                if let Some(key) = Self::string_to_key(&lower_part) {
                    modifier_keys.push(key);
                }
            } else {
                match lower_part.as_str() {
                    "left" => mouse_buttons.push(Button::Left),
                    "right" => mouse_buttons.push(Button::Right),
                    "middle" => mouse_buttons.push(Button::Middle),
                    _ => {
                        if let Some(key) = Self::string_to_key(&lower_part) {
                            action_keys.push(key);
                        }
                    }
                }
            }
        }

        match direction {
            Press => {
                // Press all modifiers first
                for &key in &modifier_keys {
                    enigo.key(key, Press)?;
                }

                // Determine the action for keys and buttons
                let action_direction = if !modifier_keys.is_empty() { Click } else { Press };

                // Press action keys
                for &key in &action_keys {
                    enigo.key(key, action_direction)?;
                }

                // Press mouse buttons
                for &button in &mouse_buttons {
                    enigo.button(button, action_direction)?;
                }
            }
            Release => {
                // Release action keys and mouse buttons only if they were pressed without modifiers
                if modifier_keys.is_empty() {
                    for &key in action_keys.iter().rev() {
                        enigo.key(key, Release)?;
                    }
                    for &button in mouse_buttons.iter().rev() {
                        enigo.button(button, Release)?;
                    }
                }

                // Release all modifiers last, in reverse order
                for &key in modifier_keys.iter().rev() {
                    enigo.key(key, Release)?;
                }
            }
            Click => {
                // This case handles a full click sequence, often used for actions combined with modifiers.
                // Press modifiers
                for &key in &modifier_keys {
                    enigo.key(key, Press)?;
                }

                // Click action keys
                for &key in &action_keys {
                    enigo.key(key, Click)?;
                }

                // Click mouse buttons
                for &button in &mouse_buttons {
                    enigo.button(button, Click)?;
                }

                // Release modifiers
                for &key in modifier_keys.iter().rev() {
                    enigo.key(key, Release)?;
                }
            }
        }

        Ok(())
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
            let (x, y) = self.enigo.location().unwrap();
            self.precision_mode_start_x = x;
            self.precision_mode_start_y = y;
        }

        let (screen_width, screen_height) = self.enigo.main_display().unwrap();

        if is_precision_mode_active {
            // --- Precision Mode: Relative movement based on the initial state ---

            // 1. Calculate the angular deviation from the center point.
            let delta_yaw = horizontal_deg - self.precision_mode_center_yaw;
            let delta_pitch = vertical_deg - self.precision_mode_center_pitch;

            // 2. Define sensitivity for precision mode. A larger FOV means slower movement.
            const PRECISION_MODE_SENSITIVITY_FACTOR: f32 = 10.0;
            let effective_fov = self.mouse_config.air_mouse_fov * PRECISION_MODE_SENSITIVITY_FACTOR;
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
            let x_ratio = (horizontal_deg / self.mouse_config.air_mouse_fov) + 0.5;
            let aspect_ratio = screen_height as f32 / screen_width as f32;
            let vertical_fov = self.mouse_config.air_mouse_fov * aspect_ratio;
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
                (speed_sq - self.mouse_config.touchpad_acceleration_threshold).max(0.0);
            let acceleration_multiplier =
                1.0 + (effective_speed_sq * 500.0 * self.mouse_config.touchpad_acceleration);
            let base_dx = delta_x * self.mouse_config.touchpad_sensitivity;
            let base_dy = delta_y * self.mouse_config.touchpad_sensitivity;

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

                let (screen_width, screen_height) = self.enigo.main_display().unwrap();
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
            let (current_x, current_y) = self.enigo.location().unwrap();
            self.target_screen_x = current_x;
            self.target_screen_y = current_y;

            // Reset sub-pixel accumulators
            self.remainder_x = 0.0;
            self.remainder_y = 0.0;

            return;
        }

        let (current_x, current_y) = self.enigo.location().unwrap();
        let dx = self.target_screen_x - current_x;
        let dy = self.target_screen_y - current_y;

        // If the distance is negligible, snap to the target to prevent jitter.
        if dx.abs() < 1 && dy.abs() < 1 {
            if current_x != self.target_screen_x || current_y != self.target_screen_y {
                if let Err(e) = self
                    .enigo
                    .move_mouse(self.target_screen_x, self.target_screen_y, Coordinate::Abs)
                {
                    eprintln!("Failed to move mouse to target position: {:?}", e);
                }
            }
            return;
        }

        // Smoothly interpolate towards the target position.
        const SMOOTHING_FACTOR: f32 = 0.3;
        let new_x = current_x + (dx as f32 * SMOOTHING_FACTOR) as i32;
        let new_y = current_y + (dy as f32 * SMOOTHING_FACTOR) as i32;

        if let Err(e) = self.enigo.move_mouse(new_x, new_y, Coordinate::Abs) {
            eprintln!("Failed to move mouse to target position: {:?}", e);
        }
    }
}