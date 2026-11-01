use super::super::LogMode;
use super::super::MWFLog;
use crate::MWF;
use crate::framework_error::MWFError;
use crate::framework_error::MWFPanic;
use crate::mwf::MWF_LOG_DATA;
use crate::mwf::MWF_LOG_MODE;
use crate::worker::WorkerID;
use std::sync::atomic::Ordering;

impl MWFLog {
    /// Change the current MWF logging mode.
    ///
    /// When LogSelected is used, the provided worker list
    /// replaces the previously configured selection.
    ///
    /// Returns Framework_NotInitialized if MWF has not
    /// been initialized through MWF::init().
    pub fn set_log_mode(log_mode: LogMode, logged_worker_list: Option<Vec<WorkerID>>) -> Result<(), MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        let mut mwf_log_data_guard = MWF_LOG_DATA.write().expect(MWFPanic::MWFLogData_WriteLockPoisoned.message());
        let mwf_log_data = mwf_log_data_guard.as_mut().ok_or(MWFError::Framework_NotInitialized)?;

        MWF_LOG_MODE.store(log_mode as u8, Ordering::Relaxed);

        if let Some(logged_worker_list) = logged_worker_list {
            mwf_log_data.logged_worker_list = logged_worker_list;
        }

        drop(mwf_log_data_guard);
        Ok(())
    }
}
