use super::super::MWF;
use super::super::MWF_WRK_DATA;
use crate::mwf::MWFError;
use crate::mwf::MWFPanic;
use crate::mwf_wrk::WorkerData;
use crate::worker_task::current_worker_id;
use std::sync::RwLockReadGuard;

impl MWF {
    /// Validate that MWF is initialized and that the current worker is still registered.
    ///
    /// Calls made outside an MWF worker task only check framework initialization.
    ///
    /// Returns the worker data read guard so callers can reuse it.
    pub(crate) fn validate_api_context() -> Result<RwLockReadGuard<'static, Option<WorkerData>>, MWFError> {
        // Check framework initialization.
        if !MWF::is_initialized() {
            return Err(MWFError::Framework_NotInitialized);
        }

        // Acquire worker data and validate the current worker when applicable.
        let mwf_wrk_data_guard = MWF_WRK_DATA.read().expect(MWFPanic::MWFWrkData_ReadLockPoisoned.message());
        let mwf_wrk_data = mwf_wrk_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;

        if let Some(worker_id) = current_worker_id() {
            mwf_wrk_data.worker_registry.get_registry_entry(&worker_id)?;
        }

        Ok(mwf_wrk_data_guard)
    }
}
