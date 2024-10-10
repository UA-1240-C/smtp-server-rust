#[cfg(test)]
mod tests {
    use crossbeam::channel::{bounded, Sender};
    use logger::{get_logger_level, initialize_logger, is_logger_running, terminate, update_severity_level, LogLevel, Logger};
    use logger::targets::{LogTarget};

    // Mock implementation of LogTarget for testing
    struct MockLogTarget {
        log_sender: Sender<String>,
    }

    impl MockLogTarget {
        fn new(sender: Sender<String>) -> Self {
            Self {
                log_sender: sender,
            }
        }
    }

    impl LogTarget for MockLogTarget {
        fn log(&self, message: String) {
            // Send the log message to the channel
            let _ = self.log_sender.send(message);
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[test]
    fn test_logger_creation() {
        let logger = Logger::new(LogLevel::Info, 10, Box::new(MockLogTarget::new(bounded(10).0)));
        assert_eq!(logger.get_log_level(), LogLevel::Info);
        assert_eq!(logger.queue_capacity.load(std::sync::atomic::Ordering::Relaxed), 10);
        assert!(logger.is_running.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn test_update_severity_level() {
        initialize_logger(
            LogLevel::Info,
            10,
            Box::new(MockLogTarget::new(bounded(10).0)),
        );
        update_severity_level(LogLevel::Debug);
        assert_eq!(get_logger_level(), LogLevel::Debug);
    }

    #[test]
    fn test_terminate_logger() {
        initialize_logger(
            LogLevel::Info,
            10,
            Box::new(MockLogTarget::new(bounded::<String>(10).0)),
        );
        terminate();
        assert!(!is_logger_running());
    }

    #[test]
    fn test_mock_log_target() {
        let (sender, receiver) = bounded::<String>(10);
        let mock_target = MockLogTarget::new(sender);
        mock_target.log("Mock log message".to_string());
        assert_eq!(receiver.recv().unwrap(), "Mock log message");
    }

    #[test]
    fn test_log_level_enum() {
        assert_eq!(LogLevel::Info as u8, 0);
        assert_eq!(LogLevel::Warn as u8, 1);
        assert_eq!(LogLevel::Error as u8, 2);
        assert_eq!(LogLevel::Debug as u8, 3);
        assert_eq!(LogLevel::Trace as u8, 4);
    }
}