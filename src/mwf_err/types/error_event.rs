use super::ErrorInstanceID;
use crate::mwf_err::terminal_error_column;
use crate::mwf_wrk::types::EntityID;
use crate::string_helper;

/// Represents an error reported to MWFErr.
///
/// Errors returned by worker lifecycle and message callbacks are recorded
/// automatically. Application code can also publish additional errors through
/// MWFErr::add_error().
///
/// Error events are stored in a circular buffer and can later be retrieved
/// for diagnostics, logging or monitoring.
#[derive(Debug, Clone)]
pub struct ErrorEvent {
    /// Unique identifier assigned by MWFErr.
    pub error_instance_id: ErrorInstanceID,

    /// Timestamp when the error was recorded, formatted according to the
    /// configured timestamp format (local time or UTC) using RFC 3339.
    pub timestamp_rfc3339: String,

    /// Origin of the error.
    pub entity_id: EntityID,

    /// Main description of the reported error.
    pub error_description: String,

    /// Chain of underlying error causes.
    pub error_context_list: Vec<String>,
}

impl ErrorEvent {
    /// Converts the error event into a human-readable terminal string.
    pub(crate) fn to_terminal_string(&self) -> String {
        let root_error_context = string_helper::error_root_context(&self.error_context_list);
        format!(
            "{:0width_id$} | {:width_time$} | {:width_entity_id$} | {:width_description$} | {:width_context$}",
            self.error_instance_id,
            &self.timestamp_rfc3339[11..23],
            self.entity_id.to_short_string(terminal_error_column::ERROR_SOURCE_ENTITY_ID_LEN),
            string_helper::to_short_string(&self.error_description, terminal_error_column::ERROR_DESCRIPTION_LEN),
            string_helper::to_short_string(&root_error_context, terminal_error_column::ERROR_CONTEXT_LEN),
            width_id = terminal_error_column::ERROR_ID_LEN,
            width_time = terminal_error_column::TIME_LEN,
            width_entity_id = terminal_error_column::ERROR_SOURCE_ENTITY_ID_LEN,
            width_description = terminal_error_column::ERROR_DESCRIPTION_LEN,
            width_context = terminal_error_column::ERROR_CONTEXT_LEN,
        )
    }

    /// Converts the error event into a CSV row.
    pub(crate) fn to_csv_string(&self) -> String {
        let error_context_list = string_helper::error_context_list_to_string(&self.error_context_list);
        format!("{},{},{},{},{}", self.error_instance_id, string_helper::csv_escape(&self.timestamp_rfc3339), string_helper::csv_escape(&self.entity_id.to_string()), string_helper::csv_escape(&self.error_description), string_helper::csv_escape(&error_context_list),)
    }
}
