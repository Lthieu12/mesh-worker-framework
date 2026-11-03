use super::super::MWFWrk;
use crate::mwf::MWF;
use crate::mwf::MWF_DATA;
use crate::mwf::MWFError;
use crate::mwf::MWFPanic;
use crate::mwf_wrk::types::WorkerID;

impl MWFWrk {
    /// Returns the fully qualified Rust type path associated with a worker.
    ///
    /// This function is intended for diagnostics and debugging. It can help
    /// locate the worker implementation in the application's source code.
    ///
    /// The returned path is an implementation detail and should not be used
    /// for application logic.
    ///
    /// Returns:
    /// - Framework_NotInitialized if MWF has not been initialized.
    /// - Worker_NotFound if the worker does not exist.
    pub fn get_worker_type_rust_path(worker_id: &WorkerID) -> Result<&'static str, MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        let mwf_wrk_data_guard = MWF::validate_api_context()?;

        // Get the WorkerTypeID associated with the worker.
        let mwf_wrk_data = mwf_wrk_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;
        let worker_type_id = mwf_wrk_data.worker_registry.get_worker_type_id(worker_id)?;
        drop(mwf_wrk_data_guard);

        // Get the Rust type name associated with the WorkerTypeID.
        let mwf_data_guard = MWF_DATA.read().expect(MWFPanic::MWFData_ReadLockPoisoned.message());
        let mwf_data = mwf_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;
        let worker_rust_type_name = mwf_data.worker_type_registry.get_worker_rust_type_name(&worker_type_id)?;

        Ok(worker_rust_type_name)
    }
}
