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
