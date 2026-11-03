use super::super::MWFLog;
use crate::mwf::MWF_LOG_DATA;
use crate::mwf::MWFError;
use crate::mwf::MWFPanic;
use crate::mwf_wrk::types::WorkerID;

impl MWFLog {
    /// Remove a worker from the MWFLog selected worker list.
    ///
    /// If the worker is not currently present in the
    /// selected worker list, this function does nothing.
    ///
    /// Returns Framework_NotInitialized if MWF has not
    /// been initialized through MWF::init().
    pub(crate) fn remove_logged_worker(worker_id: &WorkerID) -> Result<(), MWFError> {
        let mut mwf_log_data_guard = MWF_LOG_DATA.write().expect(MWFPanic::MWFLogData_WriteLockPoisoned.message());
        let mwf_log_data = mwf_log_data_guard.as_mut().ok_or(MWFError::Framework_NotInitialized)?;
        mwf_log_data.logged_worker_list.retain(|logged_worker_id| logged_worker_id != worker_id);
        drop(mwf_log_data_guard);

        Ok(())
    }
}
