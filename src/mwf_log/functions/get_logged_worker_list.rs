use super::super::MWFLog;
use crate::MWF;
use crate::mwf::MWF_LOG_DATA;
use crate::mwf::MWFError;
use crate::mwf::MWFPanic;
use crate::mwf_wrk::types::WorkerID;

impl MWFLog {
    /// Retrieve the currently selected worker list.
    ///
    /// Returns Framework_NotInitialized if MWF has not
    /// been initialized through MWF::init().
    pub fn get_logged_worker_list() -> Result<Vec<WorkerID>, MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        let mwf_log_data_guard = MWF_LOG_DATA.read().expect(MWFPanic::MWFLogData_ReadLockPoisoned.message());
        let mwf_log_data = mwf_log_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;
        let logged_worker_list = mwf_log_data.logged_worker_list.clone();

        drop(mwf_log_data_guard);
        Ok(logged_worker_list)
    }
}
