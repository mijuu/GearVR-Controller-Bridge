pub mod controller_config;
pub mod keymap_config;
pub mod mouse_config;

use serde::{Deserialize, Serialize};

use crate::config::controller_config::ControllerConfig;
use crate::config::keymap_config::KeymapConfig;
use crate::config::mouse_config::MouseConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub controller: ControllerConfig,
    pub mouse: MouseConfig,
    pub keymap: KeymapConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            controller: ControllerConfig::default(),
            mouse: MouseConfig::default(),
            keymap: KeymapConfig::default(),
        }
    }
}
