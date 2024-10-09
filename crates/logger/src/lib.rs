mod logger;

use crossbeam_queue::ArrayQueue;
use std::any::Any;
use std::{
    io::{IsTerminal, Write},
    sync::Arc
};
use std::ops::Deref;
use std::sync::LazyLock;

pub use logger::*;
use crate::msg_fmt::*;
use crate::targets::*;
use crate::writer::*;

mod logger_macro;
mod writer;
pub mod targets;
mod msg_fmt;

const DEFAULT_LOG_CAPACITY: usize = 1000;

static LOGGING_QUEUE: LazyLock<Arc<ArrayQueue<LogMessage>>> = LazyLock::new(||{
    Arc::new(ArrayQueue::new(DEFAULT_LOG_CAPACITY)
    )});

static mut LOGGER : LazyLock<Arc<Logger>> = LazyLock::new(||{Arc::new(Logger::new(LogLevel::Info,
                                                                              DEFAULT_LOG_CAPACITY,
                                                                              Box::new(ConsoleLogTarget)))});
pub fn initialize_logger(
    severity_level: LogLevel,
    queue_capacity: usize,
    target: Box<dyn LogTarget + Send + Sync>,
) {
    unsafe {
        LOGGER.update_severity_level(severity_level);
        LOGGER.update_queue_capacity(queue_capacity);
        LOGGER.add_target(target);
    }
    start_consumer_thread();
}

pub fn log(level: LogLevel, message: String) {
    unsafe { LOGGER.log(level, message); }
}

pub fn log_debug(message: String) {
    unsafe { LOGGER.log(LogLevel::Debug, message); }
}

pub fn log_prod(message: String) {
    unsafe { LOGGER.log(LogLevel::Info, message); }
}

pub fn log_prod_error(message: String) {
    unsafe { LOGGER.log(LogLevel::Error, message); }
}

pub fn log_warn(message: String) {
    unsafe { LOGGER.log(LogLevel::Warn, message); }
}

pub fn log_trace(message: String) {
    unsafe { LOGGER.log(LogLevel::Trace, message); }
}

pub fn flush() {}

pub fn terminate() {
    unsafe { LOGGER.shutdown(); }
}

pub fn set_logger_target(target: Box<dyn LogTarget + Send + Sync>) {
    unsafe { LOGGER.add_target(target); }
}

pub fn get_logger_level() -> LogLevel {
    unsafe { LOGGER.get_log_level() }
}
