use super::{ErrorEvent, ErrorInstanceID, ErrorOutputFormat};
use crate::mwf_err::terminal_error_column;

/// Snapshot of error events retrieved from MWFErr.
///
/// This structure contains the requested error events together with metadata
/// describing the state of the circular buffer.
#[derive(Debug, Clone)]
pub struct ErrorList {
    pub first_error_instance_id: Option<ErrorInstanceID>,

    pub last_error_instance_id: Option<ErrorInstanceID>,

    /// Number of events that could not be returned
    /// because they were overwritten in the circular buffer.
    pub missing_error_count: u64,

    pub error_event_list: Vec<ErrorEvent>,
}

impl std::fmt::Display for ErrorList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        writeln!(f, "{}", Self::get_header(ErrorOutputFormat::Terminal))?;

        for line in self.to_string_list(ErrorOutputFormat::Terminal) {
            writeln!(f, "{line}")?;
        }
        Ok(())
    }
}

impl ErrorList {
    /// Returns the column header corresponding to the requested format.
    pub fn get_header(format: ErrorOutputFormat) -> String {
        match format {
            ErrorOutputFormat::Terminal => {
                let header = format!(
                    "{:<width_id$} | {:<width_time$} | {:<width_entity_id$} | {:<width_description$} | {:<width_context$}",
                    "ErrorID",
                    "Time",
                    "ErrorSource/EntityID",
                    "ErrorDescription",
                    "ErrorContext",
                    width_id = terminal_error_column::ERROR_ID_LEN,
                    width_time = terminal_error_column::TIME_LEN,
                    width_entity_id = terminal_error_column::ERROR_SOURCE_ENTITY_ID_LEN,
                    width_description = terminal_error_column::ERROR_DESCRIPTION_LEN,
                    width_context = terminal_error_column::ERROR_CONTEXT_LEN,
                );

                let separator = "-".repeat(header.len());
                format!("{header}\n{separator}")
            }

            ErrorOutputFormat::CSV => String::from("ErrorID,Time,ErrorSource/EntityID,ErrorDescription,ErrorContext"),
        }
    }

    /// Converts all error events into strings.
    pub fn to_string_list(&self, format: ErrorOutputFormat) -> Vec<String> {
        match format {
            ErrorOutputFormat::Terminal => self.error_event_list.iter().map(|error_event| error_event.to_terminal_string()).collect(),

            ErrorOutputFormat::CSV => self.error_event_list.iter().map(|error_event| error_event.to_csv_string()).collect(),
        }
    }
}
