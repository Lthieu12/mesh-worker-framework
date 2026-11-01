use super::super::MWFWrk;
use crate::framework_error::MWFError;
use crate::mwf::MWF;
use crate::worker::EntityID;
use crate::worker::WorkerID;

impl MWFWrk {
    /// Get the ParentID of a worker.
    pub fn get_parent_id(worker_id: &WorkerID) -> Result<EntityID, MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        let mwf_wrk_data_guard = MWF::validate_api_context()?;

        let mwf_data = mwf_wrk_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;
        let parent_id = mwf_data.worker_registry.get_parent_id(worker_id)?;

        Ok(parent_id)
    }
}
