pub mod controller_config;
pub mod mouse_mapper_config;

use serde::{Deserialize, Serialize};

use crate::config::controller_config::ControllerConfig;
use crate::config::mouse_mapper_config::MouseMapperConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub controller: ControllerConfig,
    pub mouse_mapper: MouseMapperConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            controller: ControllerConfig::default(),
            mouse_mapper: MouseMapperConfig::default(),
        }
    }
}
