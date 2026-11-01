use super::{LogEvent, LogInstanceID, LogOutputFormat};
use crate::mwf_log::terminal_log_column;

/// Snapshot of log events retrieved from MWFLog.
///
/// This structure contains the requested log events together with metadata
/// describing the state of the circular buffer.
#[derive(Debug, Clone)]
pub struct LogList {
    pub first_log_instance_id: Option<LogInstanceID>,

    pub last_log_instance_id: Option<LogInstanceID>,

    pub missing_log_count: u64,

    pub log_event_list: Vec<LogEvent>,
}

impl std::fmt::Display for LogList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        writeln!(f, "{}", Self::get_header(LogOutputFormat::Terminal))?;

        for line in self.to_string_list(LogOutputFormat::Terminal) {
            writeln!(f, "{line}")?;
        }
        Ok(())
    }
}

impl LogList {
    /// Returns the column header corresponding to the requested format.
    pub fn get_header(format: LogOutputFormat) -> String {
        match format {
            LogOutputFormat::Terminal => {
                let header = format!(
                    "{:<width_log$} | {:<width_time$} | {:<width_worker$} | {:<width_event$} | {:<width_message$} | {:<width_sender$} | P | W | {:<width_status$} | {:<width_duration$}",
                    "LogID",
                    "Time",
                    "WorkerID",
                    "Event",
                    "MessageID",
                    "SenderID",
                    "STATUS",
                    "Duration",
                    width_log = terminal_log_column::LOG_ID_LEN,
                    width_time = terminal_log_column::TIME_LEN,
                    width_worker = terminal_log_column::WORKER_ID_LEN,
                    width_event = terminal_log_column::EVENT_LEN,
                    width_message = terminal_log_column::MESSAGE_ID_LEN,
                    width_sender = terminal_log_column::SENDER_ID_LEN,
                    width_status = terminal_log_column::STATUS_LEN,
                    width_duration = terminal_log_column::DURATION_LEN,
                );

                let separator = "-".repeat(header.len());
                format!("{header}\n{separator}")
            }

            LogOutputFormat::CSV => String::from("LogID,Time,WorkerID,Event,MessageInstanceID,MessageID,SenderID,Priority,Wait,Status,Duration"),
        }
    }

    /// Converts all log events into strings.
    pub fn to_string_list(&self, format: LogOutputFormat) -> Vec<String> {
        match format {
            LogOutputFormat::Terminal => self.log_event_list.iter().map(|log_event| log_event.to_terminal_string()).collect(),

            LogOutputFormat::CSV => self.log_event_list.iter().map(|log_event| log_event.to_csv_string()).collect(),
        }
    }
}
