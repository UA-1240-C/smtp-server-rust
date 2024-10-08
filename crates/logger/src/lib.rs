mod logger;

use std::sync::{Arc, LazyLock};
pub use logger::*;
mod logger_macro;
mod writer;
mod queue;

static LOGGER : LazyLock<Arc<Logger>> = LazyLock::new(||{Arc::new(Logger::new(LogLevel::Trace, "serverlog.txt".to_string()))});


pub fn initialize_logger(severity_level: LogLevel, filename: String) {
    update_severity_level(severity_level);
    start_consumer_thread();
}

pub fn update_severity_level(severity_level: LogLevel) {
    LOGGER.update_severity_level(severity_level);
}

pub fn log(level: LogLevel, message: String) {
    LOGGER.log(level, message);
}

pub fn log_debug(message: String) {
    LOGGER.log(LogLevel::Debug, message);
}

pub fn log_prod(message: String) {
    LOGGER.log(LogLevel::Info, message);
}

pub fn log_warn(message: String) {
    LOGGER.log(LogLevel::Warn, message);
}

pub fn log_error(message: String) {
    LOGGER.log(LogLevel::Error, message);
}

pub fn log_trace(message: String) {
    LOGGER.log(LogLevel::Trace, message);
}

pub fn flush() {}

pub fn terminate() {
    LOGGER.shutdown();
}

pub fn set_logger_level(severity_level: LogLevel) {
    LOGGER.update_severity_level(severity_level);
}

pub fn set_logger_target(target: Box<dyn LogTarget + Send + Sync>) {}

pub fn set_logger_cache_capacity(capacity: usize) {}

pub fn get_logger_level() -> LogLevel {
    LOGGER.get_log_level()
}
