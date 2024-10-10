//! # Logger Module
//! This module defines a logging system that supports multithreaded logging using
//! crossbeam's `ArrayQueue` for buffering log messages in a queue. It allows dynamic configuration
//! of log targets (e.g., console, file) and the logging severity level.
//! The logger is designed to enqueue log messages, which are then processed by a
//! background consumer thread.

mod logger;

use crossbeam_queue::ArrayQueue;
use std::{sync::Arc};
use std::sync::LazyLock;

pub use logger::*;
use crate::msg_fmt::*;
use crate::targets::*;
use crate::writer::*;

mod logger_macro;
mod writer;
pub mod targets;
mod msg_fmt;

/// Default capacity for the log message queue.
const DEFAULT_LOG_CAPACITY: usize = 1000;

/// Static instance of the logging queue, using crossbeam's `ArrayQueue` for thread-safe
/// enqueueing of log messages. The queue size is set by `DEFAULT_LOG_CAPACITY`.
///
/// `LazyLock` ensures that the queue is only initialized when first accessed.
static LOGGING_QUEUE: LazyLock<Arc<ArrayQueue<LogMessage>>> = LazyLock::new(|| {
    Arc::new(ArrayQueue::new(DEFAULT_LOG_CAPACITY))
});

/// Static logger instance, wrapped in an `Arc` for thread-safe shared ownership.
/// `LazyLock` ensures that the logger is initialized only once.
///
/// The logger is created with default settings:
/// - LogLevel set to `Trace`
/// - Queue capacity set to `DEFAULT_LOG_CAPACITY`
/// - A `ConsoleLogTarget` as the default output target.
///
/// This `LOGGER` instance is updated when `initialize_logger` is called.
static mut LOGGER: LazyLock<Arc<Logger>> = LazyLock::new(|| {
    Arc::new(Logger::new(LogLevel::Trace, DEFAULT_LOG_CAPACITY, Box::new(ConsoleLogTarget)))
});

/// Initializes the logger with the given severity level, queue capacity, and log target.
/// This function updates the global logger configuration, allowing dynamic control
/// over the logging behavior.
///
/// # Arguments:
/// * `severity_level`: Defines the minimum level of severity that will be logged.
/// * `queue_capacity`: Sets the capacity of the log message queue.
/// * `target`: A boxed log target where the messages will be sent (e.g., console, file).
///
/// # Safety:
/// This function modifies the global `LOGGER` instance using `unsafe`, so ensure
/// thread safety when calling it.
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
    // Start a consumer thread that processes log messages from the queue.
    start_consumer_thread();
}

/// Updates the severity level of the logger, changing the minimum level of severity
/// that will be logged.
/// This function allows for dynamic control over the logging behavior at runtime.
///
/// # Arguments:
/// * `new_level`: The new severity level to be set (e.g., Trace, Debug, Error).
///
/// # Safety:
/// Directly accesses the global `LOGGER` instance using `unsafe`.
pub fn update_severity_level(new_level: LogLevel) {
    unsafe { LOGGER.update_severity_level(new_level); }
}

/// Logs a message with the specified severity level.
/// The log message is enqueued and will be processed by the consumer thread.
///
/// # Arguments:
/// * `level`: The severity level of the log message (e.g., Trace, Debug, Error).
/// * `message`: The log message to be recorded.
///
/// # Safety:
/// Directly accesses the global `LOGGER` instance using `unsafe`.
pub fn log(level: LogLevel, message: String) {
    unsafe { LOGGER.log(level, message); }
}

/// Logs a message with `Debug` severity.
/// This is a convenience function for logging debug-level messages.
///
/// # Arguments:
/// * `message`: The log message to be recorded.
pub fn log_debug(message: String) {
    unsafe { LOGGER.log(LogLevel::Debug, message); }
}

/// Logs a message with `Info` severity, typically used for production-level logs.
///
/// # Arguments:
/// * `message`: The log message to be recorded.
pub fn log_prod(message: String) {
    unsafe { LOGGER.log(LogLevel::Info, message); }
}

/// Logs a message with `Error` severity, typically used for critical errors or issues.
///
/// # Arguments:
/// * `message`: The log message to be recorded.
pub fn log_error(message: String) {
    unsafe { LOGGER.log(LogLevel::Error, message); }
}

/// Logs a message with `Warn` severity, typically used for warnings or potential issues.
///
/// # Arguments:
/// * `message`: The log message to be recorded.
pub fn log_warn(message: String) {
    unsafe { LOGGER.log(LogLevel::Warn, message); }
}

/// Logs a message with `Trace` severity, typically used for tracing program execution.
///
/// # Arguments:
/// * `message`: The log message to be recorded.
pub fn log_trace(message: String) {
    unsafe { LOGGER.log(LogLevel::Trace, message); }
}

/// Flushes any buffered log messages.
///
/// This function is typically used to ensure that all pending log messages have been processed
/// and written to their targets. It may be called before program termination to avoid losing logs.
pub fn flush() {}

/// Terminates the logger by shutting down the consumer thread and ensuring that
/// all log messages are processed before exiting.
///
/// This is crucial for graceful shutdown of the logger in multi-threaded applications.
pub fn terminate() {
    unsafe { LOGGER.shutdown(); }
}

/// Sets a new target for the logger, dynamically changing where the log messages are sent.
/// This function allows for flexibility in log output, such as switching from console to file
/// logging at runtime.
///
/// # Arguments:
/// * `target`: A boxed `LogTarget` where the messages will be sent (e.g., console, file).
///
/// # Safety:
/// Directly accesses the global `LOGGER` instance using `unsafe`.
pub fn set_logger_target(target: Box<dyn LogTarget + Send + Sync>) {
    unsafe { LOGGER.add_target(target); }
}

/// Returns the current logging severity level.
/// This is useful for checking the current configuration of the logger at runtime.
///
/// # Returns:
/// The current `LogLevel` (e.g., Trace, Debug, Info).
///
/// # Safety:
/// Directly accesses the global `LOGGER` instance using `unsafe`.
pub fn get_logger_level() -> LogLevel {
    unsafe { LOGGER.get_log_level() }
}

/// Checks if the logger is currently running.
/// This function can be used to determine if the logger is active and processing log messages.
/// If the logger is not running, it may indicate an issue with the logging system.
///
/// # Returns:
/// A boolean value indicating whether the logger is running (`true`) or not (`false`).
///
/// # Safety:
/// Directly accesses the global `LOGGER` instance using `unsafe`.
pub fn is_logger_running() -> bool {
    unsafe { LOGGER.get_is_running() }
}