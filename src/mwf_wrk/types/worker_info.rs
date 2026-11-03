use crate::mwf_wrk::types::{EntityID, WorkerID};
use crate::string_helper;

use super::WorkerState;

/// Information describing a currently registered worker.
///
/// This structure provides a snapshot of the worker
/// state at the time it was retrieved.
///
/// It is primarily intended for:
/// - Debugging.
/// - Monitoring.
/// - User interfaces.
/// - Logging configuration tools.
#[derive(Debug, Clone)]
pub struct WorkerInfo {
    /// Unique identifier of the worker.
    pub worker_id: WorkerID,
    /// Identifier of the parent entity that created this worker.
    pub parent_id: EntityID,
    /// Current lifecycle state of the worker.
    pub worker_state: WorkerState,
}

impl WorkerInfo {
    /// Converts the worker information into a human-readable terminal string.
    fn to_terminal_string(&self) -> String {
        format!(
            "{:width_worker$} | {:width_father$} | {:width_state$}",
            EntityID::Worker(self.worker_id.clone()).to_short_string(terminal_worker_column::WORKER_ID_LEN,),
            self.parent_id.to_short_string(terminal_worker_column::FATHER_ID_LEN,),
            self.worker_state.as_str(),
            width_worker = terminal_worker_column::WORKER_ID_LEN,
            width_father = terminal_worker_column::FATHER_ID_LEN,
            width_state = terminal_worker_column::STATE_LEN,
        )
    }

    /// Converts the worker information into a CSV row.
    fn to_csv_string(&self) -> String {
        format!("{},{},{}", string_helper::csv_escape(self.worker_id.as_str()), string_helper::csv_escape(&self.parent_id.to_string()), string_helper::csv_escape(self.worker_state.as_str()),)
    }
}

/// Output format used to represent worker information.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WorkerOutputFormat {
    /// Human-readable terminal output.
    Terminal,
    /// Comma-separated values output.
    CSV,
}
/// Terminal worker column widths.
mod terminal_worker_column {

    pub(crate) const WORKER_ID_LEN: usize = 30;
    pub(crate) const FATHER_ID_LEN: usize = 30;
    pub(crate) const STATE_LEN: usize = 14;
}

#[path = "worker_list.rs"]
mod worker_list;

pub use worker_list::WorkerList;
