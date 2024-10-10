#![allow(dead_code)]

use std::{fs::File, path, sync::{atomic::{AtomicPtr, AtomicU32}, Arc, Mutex}};
use chrono::{DateTime, Local};

pub struct LogMessage {
    level: LogLevel,
    thread_id: std::thread::ThreadId,
    timestamp: DateTime<Local>,
    message: String,
}

impl std::fmt::Display for LogMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let uncolored = format!("[{}] [{:?}] [{:5}] {}", self.timestamp.format("%Y-%m-%d %H:%M:%S.%f"), self.thread_id, format!("{:?}", self.level), self.message);
        let colored = match self.level {
            LogLevel::Info => format!("\x1b[32m{}\x1b[0m", uncolored),
            LogLevel::Warn => format!("\x1b[33m{}\x1b[0m", uncolored),
            LogLevel::Error => format!("\x1b[31m{}\x1b[0m", uncolored),
            LogLevel::Debug => format!("\x1b[34m{}\x1b[0m", uncolored),
            LogLevel::Trace => format!("\x1b[35m{}\x1b[0m", uncolored),
        };
        write!(f, "{}", colored)
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
    fn log(&self, message: &str);
    fn flush(&mut self);
}

pub struct NoopLogTarget;

impl LogTarget for NoopLogTarget {
    fn log(&self, _message: &str) {}
    fn flush(&mut self) {}
}

pub struct ConsoleLogTarget;

use std::io::Write;

impl LogTarget for ConsoleLogTarget {
    fn log(&self, message: &str) {
        let result = write!(std::io::stdout(), "{}", message);
        if result.is_err() {
            eprintln!("Failed to write to stdout");
        }
    }
    fn flush(&mut self) {
        let result = std::io::stdout().flush();
        if result.is_err() {
            eprintln!("Failed to flush stdout");
        }
    }
}

pub struct FileLogTarget {
    file: File,
}

impl LogTarget for FileLogTarget {
    fn log(&self, message: &str) {
        use std::io::Write;
        let result = write!(&self.file, "{}", message);
        if result.is_err() {
            eprintln!("Failed to write to file");
        }
    }
    fn flush(&mut self) {
        let result = self.file.flush();
        if result.is_err() {
            eprintln!("Failed to flush file");
        }

    }
}

impl FileLogTarget {
    pub fn new(path: &path::Path) -> Self {
        let file = File::create(path).unwrap();
        FileLogTarget { file }
    }
}



pub enum LogCommand {
    Log(LogMessage),
    Flush,
    Terminate,
}

pub struct Logger {
    pub sender: crossbeam::channel::Sender<LogCommand>,
    logger_thread: Mutex<Option<std::thread::JoinHandle<()>>>,
    level: Arc<AtomicPtr<LogLevel>>,
    target: Arc<AtomicPtr<Box<dyn LogTarget + Send + Sync>>>,
    cache_capacity: Arc<AtomicU32>,
}

impl Logger {
    pub fn new(target: Box<dyn LogTarget + Send + Sync>, level: LogLevel, cache_capacity: usize) -> Self {
        let (sender, receiver) = crossbeam::channel::unbounded();

        let level_ptr = Arc::new(AtomicPtr::new(Box::into_raw(Box::new(level))));
        let target_ptr = Arc::new(AtomicPtr::new(Box::into_raw(Box::new(target))));
        let cache_capacity = Arc::new(AtomicU32::new(cache_capacity as u32));

        let logger = Logger {
            sender,
            logger_thread: Mutex::new(Some(Self::start_logger_thread(receiver, 
                target_ptr.clone(),
                level_ptr.clone(),
                cache_capacity.clone()))),
            level: level_ptr.clone(),
            target: target_ptr.clone(),
            cache_capacity: cache_capacity.clone(),
        };

        logger
    }

    pub fn log(&self, level: LogLevel, message: String) {
        let message = LogMessage {
            level,
            thread_id: std::thread::current().id(),
            timestamp: chrono::Local::now(),
            message,
        };
        match self.sender.send(LogCommand::Log(message)) {
            Ok(_) => {},
            Err(_) => eprintln!("Failed to send log message to logger thread"),
        }
    }

    fn start_logger_thread(receiver: crossbeam::channel::Receiver<LogCommand>,
        target: Arc<AtomicPtr<Box<dyn LogTarget + Send + Sync>>>,
        level: Arc<AtomicPtr<LogLevel>>,
        cache_capacity: Arc<AtomicU32>) -> std::thread::JoinHandle<()> {


        std::thread::spawn(move || {

            let mut cache = Vec::with_capacity(cache_capacity.load(std::sync::atomic::Ordering::Acquire) as usize);

            loop {
                match receiver.recv() {
                    Ok(LogCommand::Log(message)) => {
                        let current_level = level.load(std::sync::atomic::Ordering::Acquire);
                        let current_level = unsafe { &*current_level };

                        if message.level > *current_level {
                            continue;
                        }

                        cache.push(message);

                        let cache_capacity = cache_capacity.load(std::sync::atomic::Ordering::Acquire) as usize;
                        if cache.len() >= cache_capacity {
                            if let Some(target) = unsafe { target.load(std::sync::atomic::Ordering::Acquire).as_mut() } {
                                Self::flush(target, &mut cache);
                            }

                            if cache.capacity() != cache_capacity {
                                cache = Vec::with_capacity(cache_capacity);
                            }
                        }
                    }
                    Ok(LogCommand::Flush) => {
                        if let Some(target) = unsafe { target.load(std::sync::atomic::Ordering::Acquire).as_mut() } {
                            Self::flush(target, &mut cache);
                        }
                    }
                    Ok(LogCommand::Terminate) => {

                        if let Some(target) = unsafe { target.load(std::sync::atomic::Ordering::Acquire).as_mut() } {
                            Self::flush(target, &mut cache);
                        }

                        while let Ok(LogCommand::Log(message)) = receiver.try_recv() {
                            let current_level = level.load(std::sync::atomic::Ordering::Acquire);
                            let current_level = unsafe { &*current_level };

                            if message.level > *current_level {
                                continue;
                            }

                            if let Some(target) = unsafe { target.load(std::sync::atomic::Ordering::Acquire).as_ref() } {
                                target.log(&message.to_string());
                            }
                        }

                        break;
                    }
                    Err(_) => break,
                }
            }
        })
    }

    fn flush(target: &mut Box<dyn LogTarget + Send + Sync>, cache: &mut Vec<LogMessage>) {
        let combined_logs = Self::concat_cache(cache);
        target.log(&combined_logs);
        target.flush();
        cache.clear();
    }

    fn concat_cache(cache: &Vec<LogMessage>) -> String {
        cache.iter().map(|message| format!("{}\n", message.to_string())).collect()
    }

    pub fn update_level(&self, level: LogLevel) {
        let new_level_ptr = Box::into_raw(Box::new(level));
        self.level.store(new_level_ptr, std::sync::atomic::Ordering::Release);
    }

    pub fn update_target(&self, target: Box<dyn LogTarget + Send + Sync>) {
        let new_target_ptr = Box::into_raw(Box::new(target));
        self.target.store(new_target_ptr, std::sync::atomic::Ordering::Release);
    }

    pub fn update_cache_capacity(&self, capacity: usize) {
        self.cache_capacity.store(capacity as u32, std::sync::atomic::Ordering::Release);
    }

    pub fn terminate(&self) {
        let result = self.sender.send(LogCommand::Terminate);
        if result.is_err() {
            eprintln!("Failed to send terminate command to logger thread");
        }

        if let Some(logger_thread) = self.logger_thread.lock().unwrap().take() {
            if let Err(err) = logger_thread.join() {
                eprintln!("Failed to join logger thread: {:?}", err);
            }
        }
    }

    pub fn get_log_level(&self) -> LogLevel {
        let level_ptr = self.level.load(std::sync::atomic::Ordering::Acquire);
        let level = unsafe { &*level_ptr };
        *level
    }
}
