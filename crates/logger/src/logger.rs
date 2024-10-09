use chrono::{DateTime, Local};
use std::{fs::File,
          io::{Write, IsTerminal},
          sync::{atomic::{AtomicPtr, AtomicBool},
                 Arc
          },
          thread
};
use std::any::Any;
use std::path::Path;
use std::sync::atomic::AtomicU32;
use std::thread::ThreadId;
use crate::{LOGGER, LOGGING_QUEUE};

#[derive(Clone)]
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
    fn fmt(&self,
           f: &mut std::fmt::Formatter
    ) -> std::fmt::Result {
        let message_fmt = format!("{:?} - {} [{:5}] - {}",
                                  self.thread_id,
                                  self.timestamp.format("%d/%m/%Y %H:%M:%S.%f"),
                                  format!("{:?}", self.level),
                                  self.message
        );
        if std::io::stdout().is_terminal() {
            let colors = Colors::new(message_fmt);
            let colored_msg = match self.level {
                LogLevel::Info => colors.info,
                LogLevel::Warn => colors.warn,
                LogLevel::Error => colors.error,
                LogLevel::Debug => colors.debug,
                LogLevel::Trace => colors.trace,
            };
            write!(f, "{}", colored_msg)
        } else {
            write!(f, "{}", message_fmt)
        }
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
    fn as_any(&self) -> &dyn Any;
}

pub struct NoopLogTarget;

impl LogTarget for NoopLogTarget {
    fn log(&self,
           _message: String
    ) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct ConsoleLogTarget;

impl LogTarget for ConsoleLogTarget {
    fn log(&self,
           message: String
    ) {
        let result = writeln!(std::io::stdout(), "{}", message);
        match result {
            Ok(_) => {},
            Err(_) => eprintln!("Failed to write to stdout! BAD!"),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
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

    fn as_any(&self) -> &dyn Any {
        self
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
    queue_capacity: Arc<AtomicU32>,
    is_running: Arc<AtomicBool>,
    severity_level: Arc<AtomicPtr<LogLevel>>,
    targets: Arc<AtomicPtr<Vec<Box<dyn LogTarget + Send + Sync>>>>
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
}

pub fn start_consumer_thread() {
    let consumer_queue_ptr = LOGGING_QUEUE.clone();
    unsafe {
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
}