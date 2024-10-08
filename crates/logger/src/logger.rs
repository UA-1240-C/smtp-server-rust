#![allow(dead_code)]

use chrono::{DateTime, Local};
use crossbeam_queue::ArrayQueue;
use std::{fs::File,
          io::Write,
          sync::{atomic::{AtomicPtr, AtomicBool},
                 Arc
          },
          thread
};
use std::path::Path;
use std::thread::ThreadId;
use crate::LOGGER;

pub struct LogMessage {
    level: LogLevel,
    thread_id: ThreadId,
    timestamp: DateTime<Local>,
    message: String,
}

pub struct Colors {
    info: String,
    warn: String,
    error: String,
    debug: String,
    trace: String
}

impl Colors {
    fn new(message_fmt: String) -> Self {
        Colors {
            info: format!("\x1b[1;32m{}\x1b[0m", message_fmt), // green
            warn: format!("\x1b[1;33m{}\x1b[0m", message_fmt), // yellow
            error: format!("\x1b[1;31m{}\x1b[0m", message_fmt), // red
            debug: format!("\x1b[1;34m{}\x1b[0m", message_fmt), // blue
            trace: format!("\x1b[1;35m{}\x1b[0m", message_fmt) // magenta
        }
    }
}

impl std::fmt::Display for LogMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let message_fmt = format!("{:?} - {} [{:5}] - {}",
                                  self.thread_id,
                                  self.timestamp.format("%d/%m/%Y %H:%M:%S.%f"),
                                  format!("{:?}", self.level),
                                  self.message
        );
        let colors = Colors::new(message_fmt);
        let colored_msg = match self.level {
            LogLevel::Info => colors.info,
            LogLevel::Warn => colors.warn,
            LogLevel::Error => colors.error,
            LogLevel::Debug => colors.debug,
            LogLevel::Trace => colors.trace,
        };

        write!(f, "{}", colored_msg)
    }
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Clone, Copy)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
    Trace,
}

pub trait LogTarget {
    fn log(&self,
           message: String
    );
}

pub struct NoopLogTarget;

impl LogTarget for NoopLogTarget {
    fn log(&self,
           _message: String
    ) {}
}

pub struct ConsoleLogTarget;

impl LogTarget for ConsoleLogTarget {
    fn log(&self,
           message: String
    ) {
        let result = write!(std::io::stdout(), "{}", message);
        match result {
            Ok(_) => {},
            Err(_) => eprintln!("Failed to write to stdout! BAD!"),
        }
    }
}

pub struct FileLogTarget {
    file: File
}

impl LogTarget for FileLogTarget {
    fn log(&self,
           message: String
    ) {
        let result = writeln!(&self.file, "{}", message);
        match result {
            Ok(_) => {},
            Err(_) => eprintln!("Failed to write to log file!"),
        }
    }
}

impl FileLogTarget {
    pub fn new(path: &Path) -> Self {
        let file = std::fs::OpenOptions::new().append(true).create(true).open(path);
        match file {
            Ok(file) => FileLogTarget { file },
            Err(err) => {
                eprintln!("Failed to create file: {:?}! default file is generated!", err);
                FileLogTarget {
                    file: std::fs::OpenOptions::new().append(true).create(true).open("serverlog_default.txt").unwrap()
                }
            }
        }
    }
}

pub struct Logger {
    mpmc_queue: Arc<ArrayQueue<LogMessage>>,
    is_running: Arc<AtomicBool>,
    severity_level: Arc<AtomicPtr<LogLevel>>,
    targets: Vec<Box<dyn LogTarget + Send + Sync>>
}

impl Logger {
    pub fn new(
        severity_level: LogLevel,
        filename: String
    ) -> Self {
        const QUEUE_CAPACITY: usize = 1024;
        let mpmc_queue = Arc::new(ArrayQueue::new(QUEUE_CAPACITY));
        let is_running = Arc::new(AtomicBool::new(true));
        let severity_level_ptr = Arc::new(AtomicPtr::new(Box::into_raw(Box::new(severity_level))));
        let filepath = Path::new(&filename);
        let targets: Vec<Box<dyn LogTarget + Send + Sync>> = vec![
            Box::new(ConsoleLogTarget),
            Box::new(FileLogTarget::new(filepath)),
            // Box::new(SyslogTarget)
        ];

        let logger = Logger {
            mpmc_queue,
            is_running,
            severity_level: severity_level_ptr,
            targets
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
        if self.mpmc_queue.is_full() {
            eprintln!("Logger queue is full! BAD!");
            return;
        }
        match self.mpmc_queue.push(log_message) {
            Ok(_) => {}
            Err(_) => eprintln!("Failed to push log to the queue! BAD!"),
        };
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
        for target in self.targets.iter() {
            target.log(log_message.to_string());
        }
    }

    pub fn update_severity_level(
        &self,
        level: LogLevel
    ) -> () {
        let new_level_ptr = Box::into_raw(Box::new(level));
        self.severity_level.store(new_level_ptr, std::sync::atomic::Ordering::Release);
    }

    // TODO: implement this function properly: rn theres a conflict with
    // static logger and mutability of targets
    pub fn update_filename(
        &mut self,
        filename: String
    ) -> () {
        let filepath = Path::new(&filename);
        let new_target = Box::new(FileLogTarget::new(filepath));
        let new_target_ptr = Box::into_raw(new_target);
        self.targets[1] = unsafe { Box::from_raw(new_target_ptr) };
    }

    pub fn shutdown(&self) -> () {
        self.is_running.store(false, std::sync::atomic::Ordering::Release);
        while let Some(msg) = self.mpmc_queue.pop() {
            self.log_write(&msg);
        }
    }

    pub fn get_log_level(&self) -> LogLevel {
        let level_ptr = self.severity_level.load(std::sync::atomic::Ordering::Acquire);
        let level = unsafe { &*level_ptr };
        *level
    }
}

pub fn start_consumer_thread() {
    let consumer_queue_ptr = LOGGER.mpmc_queue.clone();
    let consumer_is_running_ptr = LOGGER.is_running.clone();
    let consumer_severity_level_ptr = LOGGER.severity_level.clone();

    thread::spawn(move || unsafe {
        const BATCH_SIZE: usize = 32;
        let severity_to_compare = consumer_severity_level_ptr.load(std::sync::atomic::Ordering::Acquire);
        while consumer_is_running_ptr.load(std::sync::atomic::Ordering::Acquire)
            || !consumer_queue_ptr.is_empty() {
            while let Some(msg) = consumer_queue_ptr.pop() {
                if msg.level < *severity_to_compare {
                    continue;
                }
                Logger::log_write(&**LOGGER,
                                  &msg
                );
            }
        }
    });

}
