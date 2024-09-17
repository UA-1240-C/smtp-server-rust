use json_parser::JsonParser;
use std::{
    io::Read,
    fs::File,
    path::Path,
};

use logger::{info, warn, ConsoleLogTarget, FileLogTarget, LogLevel, LogTarget};

pub struct Config {
    pub ip: String,
    pub port: u16,
    pub log_level: LogLevel,
    pub log_target: Box<dyn logger::LogTarget + Send + Sync + 'static>,
    pub capacity: usize,
    pub pool_size: usize,
    pub timeout: u64,
}

impl Default for Config {
    fn default() -> Self {
        let mut parser = JsonParser::default();
        let mut raw_config = String::new();
        File::open("config.json").unwrap().read_to_string(&mut raw_config).unwrap();
        let config_obj = parser.parse(&raw_config).unwrap();

        let ip = match config_obj["server"]["ip-address"].as_str() {
            Some(ip) => {
                ip
            },
            None => {
                warn!("IP address not found, using default");
                "127.0.0.1".to_string()
            }
        };
        info!("IP address: {}", ip);

        let port = match config_obj["server"]["port"].as_number() {
            Some(port) => {
                port as u16
            },
            None => {
                warn!("Port not found, using default");
                2525
            }
        };
        info!("Port: {}", port);

        let log_level = match config_obj["logging"]["log-level"].as_str() {
            Some(level) => match level.as_str() {
                "trace" => LogLevel::Trace,
                "debug" => LogLevel::Debug,
                "info" => LogLevel::Info,
                "warn" => LogLevel::Warn,
                "error" => LogLevel::Error,
                _ => {
                    warn!("Invalid log level, using default");
                    LogLevel::Info
                },
            },
            None => {
                warn!("Log level not found, using default");
                LogLevel::Info
            },
        };
        info!("Log level: {:?}", log_level);

        let capacity = match config_obj["logging"]["cache-capacity"].as_number() {
            Some(capacity) => {
                capacity as usize
            },
            None => {
                warn!("Cache capacity not found, using default");
                1000
            }
        };
        info!("Cache capacity: {}", capacity);

        let pool_size = match config_obj["thread-pool"]["pool-size"].as_number() {
            Some(pool_size) => {
                pool_size as usize
            },
            None => {
                warn!("Thread pool size not found, using default");
                10
            }
        };
        info!("Thread pool size: {}", pool_size);

        let log_target: Box<dyn LogTarget + Send + Sync + 'static> =
        match config_obj["logging"]["log-target"].as_str().unwrap_or("console".to_string()).as_str() {
            "console" => {
                info!("Log target: console");
                Box::new(ConsoleLogTarget)
            },
            "file" => {
                let file_path = config_obj["logging"]["file-path"].as_str().unwrap_or("log.txt".to_string());
                info!("Log target: file");
                info!("File path: {}", file_path);
                Box::new(FileLogTarget::new(Path::new(&file_path)))
            }
            _ => Box::new(ConsoleLogTarget),
        };

        let timeout = match config_obj["communication"]["max-connection-timeout"].as_number() {
            Some(timeout) => {
                timeout as u64
            },
            None => {
                warn!("Timeout not found, using default");
                60_u64
            }
        };
        info!("Timeout: {}", timeout);

        Self {
            ip: ip.to_string(),
            port,
            log_level,
            log_target,
            capacity,
            pool_size,
            timeout,
        }
    }
}
