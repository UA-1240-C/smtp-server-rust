use chrono::Local;
use std::{sync::{atomic::{AtomicPtr, AtomicBool},
                 Arc
          },
          thread
};
use std::path::Path;
use std::sync::atomic::AtomicU32;

use crate::{LOGGING_QUEUE};
use crate::msg_fmt::*;
use crate::targets::*;


#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Clone, Copy)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
    Trace,
}

pub struct Logger {
    pub queue_capacity: Arc<AtomicU32>,
    pub is_running: Arc<AtomicBool>,
    pub(crate) severity_level: Arc<AtomicPtr<LogLevel>>,
    pub targets: Arc<AtomicPtr<Vec<Box<dyn LogTarget + Send + Sync>>>>
}

impl Logger {
    pub fn new(
        severity_level: LogLevel,
        queue_capacity: usize,
        target: Box<dyn LogTarget + Send + Sync>
    ) -> Self {
        let is_running = Arc::new(AtomicBool::new(true));
        let severity_level_ptr = Arc::new(AtomicPtr::new(Box::into_raw(Box::new(severity_level))));

        let mut targets: Vec<Box<dyn LogTarget + Send + Sync>> = vec![];
        if target.as_any().downcast_ref::<ConsoleLogTarget>().is_some() {
            const DEFAULT_LOG_FILENAME: &str = "serverlog.txt";
            let default_log_filename = DEFAULT_LOG_FILENAME.to_string();
            let filepath = Path::new(&default_log_filename);

            targets = vec![
                target,
                Box::new(FileLogTarget::new(filepath))
            ];
        } else {
            targets = vec![
                Box::new(ConsoleLogTarget),
                target
            ];
        }

        let targets_ptr = Arc::new(AtomicPtr::new(Box::into_raw(Box::new(targets))));

        let logger = Logger {
            queue_capacity: Arc::new(AtomicU32::new(queue_capacity as u32)),
            is_running,
            severity_level: severity_level_ptr,
            targets: targets_ptr
        };
        logger
    }

    pub fn log(
        &self,
        log_level: LogLevel,
        message: String
    ) {
        let log_message = LogMessage {
            level: log_level,
            thread_id: thread::current().id(),
            timestamp: Local::now(),
            message: message.to_string()
        };

        match LOGGING_QUEUE.push(log_message) {
            Ok(_) => {},
            Err(_) => eprintln!("Queue is full! Failed to push log to the queue!")
        }
    }

    pub fn log_write(
        &self,
        log_message: &LogMessage
    ) -> ()  {
        let task_severity_level = log_message.level;
        let filter_severity_level = unsafe {
            let sl = self.severity_level.load(std::sync::atomic::Ordering::Acquire);
            *sl
        };
        if task_severity_level > filter_severity_level {
            return;
        }
        unsafe {
            let targets_ptr = self.targets.load(std::sync::atomic::Ordering::Acquire);
            for target in (*targets_ptr).iter() {
                target.log(log_message.to_string());
            }
            syslog_message(log_message.clone());
        }
    }

    pub fn update_severity_level(
        &self,
        level: LogLevel
    ) -> () {
        let new_level_ptr = Box::into_raw(Box::new(level));
        self.severity_level.store(new_level_ptr, std::sync::atomic::Ordering::Release);
    }

    pub fn add_target(
        &self,
        target: Box<dyn LogTarget + Send + Sync>
    ) -> () {
        if target.as_any().downcast_ref::<ConsoleLogTarget>().is_some() {
            return;
        } else {
            let targets_vec = Box::into_raw(Box::new(vec![
                Box::new(ConsoleLogTarget),
                target
            ]));
            self.targets.store(targets_vec, std::sync::atomic::Ordering::Release);
        }
    }

    pub fn update_queue_capacity(&self, capacity: usize) {
        self.queue_capacity.store(capacity as u32, std::sync::atomic::Ordering::Release);
    }

    pub fn shutdown(&self) -> () {
        self.is_running.store(false, std::sync::atomic::Ordering::Release);
        while let Some(msg) = LOGGING_QUEUE.pop() {
            self.log_write(&msg);
        }
    }

    pub fn get_log_level(&self) -> LogLevel {
        let level_ptr = self.severity_level.load(std::sync::atomic::Ordering::Acquire);
        let level = unsafe { &*level_ptr };
        *level
    }

    pub fn get_is_running(&self) -> bool {
        self.is_running.load(std::sync::atomic::Ordering::Acquire)
    }
}