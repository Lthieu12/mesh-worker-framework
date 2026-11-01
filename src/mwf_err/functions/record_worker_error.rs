use super::super::MWFErr;
use crate::worker::EntityID;
use crate::worker::WorkerID;

impl MWFErr {
    /// Records a worker error without allowing error reporting to interrupt worker execution.
    pub(crate) fn record_worker_error(worker_id: &WorkerID, callback_name: Option<String>, error: &anyhow::Error) {
        // Error reporting is best-effort and must never affect the worker lifecycle.
        let _ = MWFErr::add_error_internal(&EntityID::Worker(worker_id.clone()), callback_name, error);
    }
}
