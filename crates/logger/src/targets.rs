use std::any::Any;
use std::ffi::OsStr;
use std::fs::File;
use std::path::Path;
use std::io::Write;
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use crate::LogLevel;
use crate::msg_fmt::LogMessage;

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

pub fn syslog_message(log_message: LogMessage) {
    #[cfg(target_os = "linux")]
    {
        syslog_linux(log_message);
        return;
    }

    #[cfg(target_os = "windows")]
    {
        syslog_windows(log_message);
        return;
    }
}

#[cfg(target_os = "linux")]
pub fn syslog_linux(log_message: LogMessage) {
    use syslog::{Facility, Formatter3164};

    let formatter = Formatter3164 {
        facility: Facility::LOG_USER,
        hostname: None,
        process: "Logger".into(),
        pid: 0,
    };

    let logger = syslog::unix(formatter).expect("Failed to connect to syslog");
    let error_message = format!("Failed to send message to syslog from thread {:?}", log_message.thread_id);
    match log_message.level {
        LogLevel::Error => {
            if let Err(e) = logger.err(log_message.message) {
                eprintln!("{}: {:?}", error_message, e);
            }
        },
        LogLevel::Warn => {
            if let Err(e) = logger.warning(log_message.message) {
                eprintln!("{}: {:?}", error_message, e);
            }
        },
        LogLevel::Info => {
            if let Err(e) = logger.info(log_message.message) {
                eprintln!("{}: {:?}", error_message, e);
            }
        },
        LogLevel::Debug => {
            if let Err(e) = logger.debug(log_message.message) {
                eprintln!("{}: {:?}", error_message, e);
            }
        },
        LogLevel::Trace => {
            // Trace is treated as Debug in this case
            if let Err(e) = logger.debug(log_message.message) {
                eprintln!("{}: {:?}", error_message, e);
            }
        },
        _ => {
            if let Err(e) = logger.info(log_message.message) {
                eprintln!("{}: {:?}", error_message, e);
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub fn syslog_windows(log_message: LogMessage) {
    unsafe {
        let source_wstr: Vec<u16> = OsStr::new("Logger")
            .encode_wide()
            .chain(Some(0))
            .collect();

        let message_wstr: Vec<u16> = OsStr::new(&log_message.message)
            .encode_wide()
            .chain(Some(0))
            .collect();

        use windows_sys::Win32::System::EventLog::{RegisterEventSourceW,
                                                   DeregisterEventSource,
                                                   ReportEventW,
                                                   EVENTLOG_ERROR_TYPE,
                                                   EVENTLOG_INFORMATION_TYPE,
                                                   EVENTLOG_WARNING_TYPE,
        };
        use windows_sys::core::PCWSTR;

        let event_source = RegisterEventSourceW(
            ptr::null(), // Use the local computer
            source_wstr.as_ptr() as PCWSTR,
        );

        if event_source == 0isize {
            eprintln!("Failed to register event source");
            return;
        }

        let message_pwstr = message_wstr.as_ptr() as *const PCWSTR;

        let event_type = match log_message.level {
            LogLevel::Error => EVENTLOG_ERROR_TYPE,
            LogLevel::Warn => EVENTLOG_WARNING_TYPE,
            LogLevel::Info => EVENTLOG_INFORMATION_TYPE,
            _ => EVENTLOG_INFORMATION_TYPE // Treat Debug and Trace as information
        };
        ReportEventW(
            event_source,
            event_type,
            0, // Category
            0, // Event ID
            ptr::null_mut(), // No user SID
            1, // Number of strings
            0, // Raw data size
            message_pwstr, // The message
            ptr::null(),   // No raw data
        );
        DeregisterEventSource(event_source);
    }
}