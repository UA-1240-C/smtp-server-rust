use std::io::IsTerminal;
use std::thread::ThreadId;
use chrono::{DateTime, Local};

use crate::LogLevel;

#[derive(Clone)]
pub struct LogMessage {
    pub(crate) level: LogLevel,
    pub(crate) thread_id: ThreadId,
    pub(crate) timestamp: DateTime<Local>,
    pub(crate) message: String,
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