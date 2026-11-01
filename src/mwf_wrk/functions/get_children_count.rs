use super::super::MWFWrk;
use crate::framework_error::MWFError;
use crate::mwf::MWF;
use crate::worker::EntityID;

impl MWFWrk {
    /// Get the number of child workers of an entity.
    pub fn get_children_count(entity_id: &EntityID) -> Result<usize, MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        let mwf_wrk_data_guard = MWF::validate_api_context()?;

        // Validate the entity identifier.
        if !entity_id.is_valid() {
            return Err(MWFError::Entity_Invalid { entity_id: entity_id.clone() });
        }
        let mwf_data = mwf_wrk_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;
        let children_count = mwf_data.worker_registry.get_children_count(entity_id)?;
        drop(mwf_wrk_data_guard);
        Ok(children_count)
    }
}
