use super::super::MWFWrk;
use crate::mwf::MWF;
use crate::mwf::MWFError;
use crate::mwf_wrk::types::WorkerID;
use crate::worker_task::current_worker_id;

impl MWFWrk {
    /// Return the ID of the worker executing the current MWF task.
    ///
    /// Returns an error when called outside an MWF worker task.
    pub fn get_current_worker_id() -> Result<WorkerID, MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        match current_worker_id() {
            Some(worker_id) => Ok(worker_id),
            None => Err(MWFError::Worker_CurrentContextUnavailable),
        }
    }
}
