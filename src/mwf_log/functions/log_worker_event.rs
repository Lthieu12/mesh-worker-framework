use super::super::LogAction;
use super::super::LogStatus;
use super::super::MWFLog;
use crate::MWFErr;
use crate::worker::WorkerID;
use std::time::Instant;

impl MWFLog {
    /// Adds a worker log event without allowing logging failures to interrupt worker execution.
    pub(crate) fn log_worker_event(action_start_timer: Instant, worker_id: &WorkerID, log_action: LogAction, log_status: LogStatus) {
        if let Err(log_error) = MWFLog::add_log(action_start_timer, worker_id, log_action, log_status) {
            // Report logging failures through MWFErr when possible.
            let log_error = anyhow::Error::new(log_error);
            MWFErr::record_worker_error(worker_id, Some("MWFLog::add_log".into()), &log_error);
        }
    }
}
