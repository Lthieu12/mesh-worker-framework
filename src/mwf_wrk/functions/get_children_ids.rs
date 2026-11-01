use super::super::MWFWrk;
use crate::framework_error::MWFError;
use crate::mwf::MWF;
use crate::worker::EntityID;
use crate::worker::WorkerID;

impl MWFWrk {
    /// Get the child WorkerIDs of an entity.
    pub fn get_children_ids(entity_id: &EntityID) -> Result<Vec<WorkerID>, MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        let mwf_wrk_data_guard = MWF::validate_api_context()?;

        // Validate the entity identifier.
        if !entity_id.is_valid() {
            return Err(MWFError::Entity_Invalid { entity_id: entity_id.clone() });
        }
        let mwf_data = mwf_wrk_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;

        mwf_data.worker_registry.get_children_ids(entity_id)
    }
}
