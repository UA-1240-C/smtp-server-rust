use json_parser::JsonParser;
use std::{
    io::Read,
    fs::File,
    path::Path,
};

use logger::{LogTarget, LogLevel, ConsoleLogTarget, FileLogTarget};

pub struct Config {
    pub ip: String,
    pub port: u16,
    pub log_level: LogLevel,
    pub log_target: Box<dyn logger::LogTarget + Send + Sync + 'static>,
    pub capacity: usize,
    pub pool_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        let mut parser = JsonParser::default();
        let mut raw_config = String::new();
        File::open("config.json").unwrap().read_to_string(&mut raw_config).unwrap();
        let config_obj = parser.parse(&raw_config).unwrap();

        let ip = config_obj["server"]["ip-address"].as_str().unwrap_or("127.0.0.1".to_string());
        let port = config_obj["server"]["port"].as_number().unwrap_or(2525.0) as u16;

        let log_level = match config_obj["logging"]["log-level"].as_str().unwrap_or("debug".to_string()).as_str() {
            "trace" => LogLevel::Trace,
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "warn" => LogLevel::Warn,
            "error" => LogLevel::Error,
            _ => LogLevel::Info,
        };

        let capacity = config_obj["logging"]["cache-capacity"].as_number().unwrap_or(1.0) as usize;
        let pool_size = config_obj["thread-pool"]["pool-size"].as_number().unwrap_or(1.0) as usize;

        let log_target: Box<dyn LogTarget + Send + Sync + 'static> =
        match config_obj["logging"]["log-target"].as_str().unwrap_or("console".to_string()).as_str() {
            "console" => Box::new(ConsoleLogTarget),
            "file" => {
                let file_path = config_obj["logging"]["file-path"].as_str().unwrap_or("log.txt".to_string());
                Box::new(FileLogTarget::new(Path::new(&file_path)))
            }
            _ => Box::new(ConsoleLogTarget),
        };

        Self {
            ip: ip.to_string(),
            port,
            log_level,
            log_target,
            capacity,
            pool_size,
        }
    }
}
