use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};
use serde::Serialize;
use std::sync::Once;
use chrono::Local;
use tauri::Emitter;

static INIT: Once = Once::new();
static mut LOGGER: Option<TauriLogger> = None;

#[derive(Debug, Serialize, Clone)]
pub struct LogMessage {
    level: String,
    message: String,
    timestamp: String,
}

pub struct TauriLogger {
    app_handle: tauri::AppHandle,
    level: Level,
}

impl TauriLogger {
    pub fn new(app_handle: tauri::AppHandle, level: Level) -> Self {
        Self { app_handle, level }
    }

    pub fn init(app_handle: tauri::AppHandle, level: Level) -> Result<(), SetLoggerError> {
        let logger = TauriLogger::new(app_handle, level);
        
        unsafe {
            LOGGER = Some(logger);
            
            // 将 Level 转换为对应的 LevelFilter
            let level_filter = match level {
                Level::Error => LevelFilter::Error,
                Level::Warn => LevelFilter::Warn,
                Level::Info => LevelFilter::Info,
                Level::Debug => LevelFilter::Debug,
                Level::Trace => LevelFilter::Trace,
            };
            
            let result = log::set_logger(LOGGER.as_ref().unwrap())
                .map(|()| log::set_max_level(level_filter));
                
            if result.is_ok() {
                INIT.call_once(|| {});
            }
            
            result
        }
    }

    fn emit_log(&self, record: &Record) {
        let log_message = LogMessage {
            level: record.level().to_string(),
            message: record.args().to_string(),
            timestamp: Local::now().to_rfc3339(),
        };

        // 发送日志消息到前端
        if let Err(e) = self.app_handle.emit("log-message", log_message) {
            eprintln!("Failed to emit log message: {}", e);
        }
    }
}

impl log::Log for TauriLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // 同时输出到标准错误
            eprintln!("[{}] {}", record.level(), record.args());
            
            // 发送到前端
            self.emit_log(record);
        }
    }

    fn flush(&self) {}
}