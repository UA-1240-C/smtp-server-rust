mod logger; pub use logger::*;
mod logger_macro;

use std::sync::{Arc, LazyLock};

static LOGGER : LazyLock<Arc<Logger>> = LazyLock::new(||{Arc::new(Logger::new(Box::new(NoopLogTarget), LogLevel::Info, 1))});


pub fn log(level: LogLevel, message: String) {
    LOGGER.log(level, message);
}

pub fn flush() {
    LOGGER.sender.send(LogCommand::Flush).unwrap();
}

pub fn terminate() {
    LOGGER.terminate();
}

pub fn set_logger_level(level: LogLevel) {
    LOGGER.update_level(level);
}

pub fn set_logger_target(target: Box<dyn LogTarget + Send + Sync>) {
    LOGGER.update_target(target);
}

pub fn set_logger_cache_capacity(capacity: usize) {
    LOGGER.update_cache_capacity(capacity);
}

pub fn get_logger_level() -> LogLevel {
    LOGGER.get_log_level()
}

