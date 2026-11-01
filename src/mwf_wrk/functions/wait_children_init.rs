use super::super::MWFWrk;
use crate::framework_error::MWFError;
use crate::worker::EntityID;

impl MWFWrk {
    /// Wait until all direct children of an entity reach the
    /// Initialized state.
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    pub async fn wait_children_init(entity_id: &EntityID, timeout_ms: Option<u64>) -> Result<(), MWFError> {
        // Resolve the current child list and delegate the wait
        // operation to wait_worker_list_init().
        MWFWrk::wait_worker_list_init(&MWFWrk::get_children_ids(entity_id)?, timeout_ms).await
    }
}
