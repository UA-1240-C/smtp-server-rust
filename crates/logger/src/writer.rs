use std::thread;
use crate::{LOGGER, LOGGING_QUEUE};

pub fn start_consumer_thread() {
    let consumer_queue_ptr = LOGGING_QUEUE.clone();
    thread::spawn(move || unsafe {
        while LOGGER.is_running.load(std::sync::atomic::Ordering::Acquire)
            || !consumer_queue_ptr.is_empty() {
            while let Some(msg) = consumer_queue_ptr.pop() {
                if msg.level < *LOGGER.severity_level.load(std::sync::atomic::Ordering::Acquire) {
                    continue;
                }
                LOGGER.log_write(&msg);
            }
        }
    });
}