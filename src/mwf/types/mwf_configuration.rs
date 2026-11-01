use super::TimestampFormat;
use super::TimestampFormat::LocalTime;
use std::path::PathBuf;

/// MWF startup configuration.
#[derive(Debug, Clone)]
pub struct MWFConfiguration {
    /// Capacity of normal worker event queues (must be >= 1).
    pub worker_normal_channel_capacity: usize,

    /// Capacity of priority worker event queues (must be >= 1).
    pub worker_priority_channel_capacity: usize,

    /// Timestamp format used by MWF.
    pub timestamp_format: TimestampFormat,

    /// Capacity of the MWFErr circular buffer (must be >= 1).
    pub error_event_buffer_capacity: usize,

    /// Capacity of the MWFLog circular buffer (must be >= 1).
    pub log_event_buffer_capacity: usize,

    /// Path to the MWF log file.
    ///
    /// By default, MWF creates a log file next to the executable.
    /// Set to `None` to disable file logging and prevent the log file from being created.
    pub log_file_path: Option<PathBuf>,

    /// Maximum log file size in bytes before file logging is disabled (must be >= 1024).
    pub log_file_max_size_bytes: u64,
}

impl Default for MWFConfiguration {
    fn default() -> Self {
        let mut log_file_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("mwf.exe"));
        log_file_path.set_file_name("mwf.log");

        let log_file_max_size_bytes = 10 * 1024 * 1024;

        Self { worker_normal_channel_capacity: 50, worker_priority_channel_capacity: 10, timestamp_format: LocalTime, error_event_buffer_capacity: 100, log_event_buffer_capacity: 100, log_file_path: Some(log_file_path), log_file_max_size_bytes }
    }
}
