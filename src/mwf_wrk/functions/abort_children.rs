use super::super::MWFWrk;
use crate::mwf::MWFError;
use crate::mwf_wrk::types::EntityID;

impl MWFWrk {
    /// Aborts all direct child workers of an entity.
    ///
    /// This is a best-effort operation intended as a last resort when child workers cannot
    /// be closed normally.
    ///
    /// See abort_worker_list() for details.
    pub fn abort_children(entity_id: &EntityID) -> Result<(), MWFError> {
        MWFWrk::abort_worker_list(&MWFWrk::get_children_ids(entity_id)?)
    }
}
