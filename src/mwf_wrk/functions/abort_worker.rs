use super::super::MWFWrk;
use crate::mwf::MWFError;
use crate::mwf_wrk::types::WorkerID;

impl MWFWrk {
    /// Aborts a worker.
    ///
    /// This is a best-effort operation intended as a last resort when a worker cannot be closed
    /// normally.
    ///
    /// See abort_worker_list() for details.
    pub fn abort_worker(worker_id: &WorkerID) -> Result<(), MWFError> {
        MWFWrk::abort_worker_list(std::slice::from_ref(worker_id))
    }
}
