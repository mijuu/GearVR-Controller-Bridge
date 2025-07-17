use chrono::Local;
use log::{Level, LevelFilter, Metadata, Record};
use serde::Serialize;
use std::sync::OnceLock;
use tauri::Emitter;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoggerError {
    #[error("Logger already initialized")]
    AlreadyInitialized,
    #[error("Failed to set logger: {0}")]
    SetLoggerFailed(String),
}

static LOGGER: OnceLock<TauriLogger> = OnceLock::new();

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

    pub fn init(app_handle: tauri::AppHandle, level: Level) -> Result<(), LoggerError> {
        let logger = TauriLogger::new(app_handle, level);

        LOGGER
            .set(logger)
            .map_err(|_| LoggerError::AlreadyInitialized)?;

        // 将 Level 转换为对应的 LevelFilter
        let level_filter = match level {
            Level::Error => LevelFilter::Error,
            Level::Warn => LevelFilter::Warn,
            Level::Info => LevelFilter::Info,
            Level::Debug => LevelFilter::Debug,
            Level::Trace => LevelFilter::Trace,
        };

        log::set_logger(LOGGER.get().unwrap())
            .map(|()| log::set_max_level(level_filter))
            .map_err(|e| LoggerError::SetLoggerFailed(e.to_string()))?;

        Ok(())
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
