#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)*) => {
        $crate::log($level, format!($($arg)*));
    }
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::log($crate::LogLevel::Info, format!($($arg)*));
    }
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        $crate::log($crate::LogLevel::Warn, format!($($arg)*));
    }
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        $crate::log($crate::LogLevel::Error, format!($($arg)*));
    }
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        $crate::log($crate::LogLevel::Debug, format!($($arg)*));
    }
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        $crate::log($crate::LogLevel::Trace, format!($($arg)*));
    }
}