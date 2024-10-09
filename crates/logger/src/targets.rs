use std::fs::File;
use std::path::Path;
use std::io::Write;

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
        let result = writeln!(std::io::stdout(), "{}", message);
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