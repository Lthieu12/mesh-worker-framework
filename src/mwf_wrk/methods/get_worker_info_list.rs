use super::super::MWFWrk;
use crate::framework_error::MWFError;
use crate::mwf::MWF;
use crate::worker::WorkerList;

impl MWFWrk {
    /// Retrieve information about all workers currently
    /// registered in MWF.
    ///
    /// The returned list contains a snapshot of the Registry
    /// state at the time of the call.
    ///
    /// This function is primarily intended for:
    /// - Debug tools.
    /// - Monitoring interfaces.
    /// - Runtime inspection.
    ///
    /// Returns Framework_NotInitialized if MWF has not
    /// been initialized through MWF::init().
    pub fn get_worker_info_list() -> Result<WorkerList, MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        let mwf_wrk_data_guard = MWF::validate_api_context()?;

        let mwf_data = mwf_wrk_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;

        Ok(WorkerList { worker_info_list: mwf_data.worker_registry.get_worker_info_list() })
    }
}
