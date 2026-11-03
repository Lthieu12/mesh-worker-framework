use super::{LogAction, LogInstanceID, LogStatus};
use crate::mwf_log::terminal_log_column;
use crate::mwf_wrk::types::WorkerID;
use crate::string_helper;

/// Single log event stored in MWFLog.
#[derive(Debug, Clone)]
pub struct LogEvent {
    /// Unique log identifier.
    pub log_instance_id: LogInstanceID,
    /// Event timestamp in RFC3339 format.
    pub timestamp_rfc3339: String,
    /// worker associated with the event.
    pub worker_id: WorkerID,
    /// Logged action.
    pub log_action: LogAction,
    /// Final action status.
    pub log_status: LogStatus,
    /// Action duration in milliseconds.
    pub duration_ms: Option<u128>,
}

impl LogEvent {
    /// Converts the log event into a human-readable terminal string.
    pub(crate) fn to_terminal_string(&self) -> String {
        let (event, sender_id, message_id, priority, wait) = match &self.log_action {
            LogAction::WorkerSpawn => ("SPAWN", "-".to_string(), "-".to_string(), "-".to_string(), "-".to_string()),
            LogAction::WorkerInit => ("INIT", "-".to_string(), "-".to_string(), "-".to_string(), "-".to_string()),
            LogAction::WorkerClose => ("CLOSE", "-".to_string(), "-".to_string(), "-".to_string(), "-".to_string()),
            LogAction::WorkerAbort => ("ABORT", "-".to_string(), "-".to_string(), "-".to_string(), "-".to_string()),
            LogAction::WorkerPanic => ("PANIC", "-".to_string(), "-".to_string(), "-".to_string(), "-".to_string()),
            LogAction::WorkerMessage { message_instance_id: _, sender_id, message_id, priority, wait_completion } => {
                ("MESSAGE", sender_id.to_short_string(terminal_log_column::SENDER_ID_LEN), message_id.to_short_string(terminal_log_column::MESSAGE_ID_LEN), if *priority { "Y" } else { "N" }.to_string(), if *wait_completion { "Y" } else { "N" }.to_string())
            }
        };

        let duration = match self.duration_ms {
            Some(duration_ms) => format!("{duration_ms}ms"),
            None => "-".to_string(),
        };

        let status = match self.log_status {
            LogStatus::Start => "START",
            LogStatus::Ok => "OK",
            LogStatus::Error => "ERR",
        };

        format!(
            "{:0width_log$} | {:width_time$} | {:width_worker$} | {:width_event$} | {:width_message$} | {:width_sender$} | {} | {} | {:width_status$} | {:width_duration$}",
            self.log_instance_id,
            &self.timestamp_rfc3339[11..23],
            self.worker_id.to_short_string(terminal_log_column::WORKER_ID_LEN),
            event,
            message_id,
            sender_id,
            priority,
            wait,
            status,
            duration,
            width_log = terminal_log_column::LOG_ID_LEN,
            width_time = terminal_log_column::TIME_LEN,
            width_worker = terminal_log_column::WORKER_ID_LEN,
            width_event = terminal_log_column::EVENT_LEN,
            width_message = terminal_log_column::MESSAGE_ID_LEN,
            width_sender = terminal_log_column::SENDER_ID_LEN,
            width_status = terminal_log_column::STATUS_LEN,
            width_duration = terminal_log_column::DURATION_LEN,
        )
    }

    /// Converts the log event into a CSV row.
    pub(crate) fn to_csv_string(&self) -> String {
        let (event, message_instance_id, sender_id, message_id, priority, wait) = match &self.log_action {
            LogAction::WorkerSpawn => ("SPAWN", "-".to_string(), "-".to_string(), "-".to_string(), "-".to_string(), "-".to_string()),
            LogAction::WorkerInit => ("INIT", "-".to_string(), "-".to_string(), "-".to_string(), "-".to_string(), "-".to_string()),
            LogAction::WorkerClose => ("CLOSE", "-".to_string(), "-".to_string(), "-".to_string(), "-".to_string(), "-".to_string()),
            LogAction::WorkerAbort => ("ABORT", "-".to_string(), "-".to_string(), "-".to_string(), "-".to_string(), "-".to_string()),
            LogAction::WorkerPanic => ("PANIC", "-".to_string(), "-".to_string(), "-".to_string(), "-".to_string(), "-".to_string()),
            LogAction::WorkerMessage { message_instance_id, sender_id, message_id, priority, wait_completion } => {
                ("MESSAGE", message_instance_id.to_string(), sender_id.to_string(), message_id.to_string(), if *priority { "Y" } else { "N" }.to_string(), if *wait_completion { "Y" } else { "N" }.to_string())
            }
        };

        let duration = match self.duration_ms {
            Some(duration_ms) => format!("{duration_ms}ms"),
            None => "-".to_string(),
        };

        let status = match self.log_status {
            LogStatus::Start => "START",
            LogStatus::Ok => "OK",
            LogStatus::Error => "ERR",
        };

        format!(
            "{},{},{},{},{},{},{},{},{},{},{}",
            self.log_instance_id,
            string_helper::csv_escape(&self.timestamp_rfc3339),
            string_helper::csv_escape(self.worker_id.as_str()),
            string_helper::csv_escape(event),
            string_helper::csv_escape(&message_instance_id),
            string_helper::csv_escape(&message_id),
            string_helper::csv_escape(&sender_id),
            string_helper::csv_escape(&priority),
            string_helper::csv_escape(&wait),
            string_helper::csv_escape(status),
            string_helper::csv_escape(&duration),
        )
    }
}
