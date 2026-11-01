use super::super::MWFWrk;
use crate::framework_error::MWFError;
use crate::mwf::MWF;
use crate::worker::EntityID;
use crate::worker_task::current_parent_id;

impl MWFWrk {
    /// Return the parent entity ID of the current worker.
    ///
    /// Returns an error when called outside an MWF worker task.
    pub fn get_current_parent_id() -> Result<EntityID, MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        match current_parent_id() {
            Some(parent_id) => Ok(parent_id),
            None => Err(MWFError::Worker_CurrentContextUnavailable),
        }
    }
}
