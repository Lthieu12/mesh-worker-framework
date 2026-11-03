use crate::mwf::MWF_MESSAGE_WAIT_DATA;
use crate::mwf_wrk::types::{MessageInstanceID, WorkerID};

/// Automatically removes a registered wait when the guard is dropped.
#[derive(Debug)]
pub(crate) struct WorkerWaitGuard {
    /// Worker whose wait is registered.
    worker_id: WorkerID,

    /// Message instance associated with this specific wait.
    message_instance_id: MessageInstanceID,
}

impl Drop for WorkerWaitGuard {
    fn drop(&mut self) {
        // Do not panic during cleanup if the registry lock is poisoned.
        let Ok(mut message_wait_data_guard) = MWF_MESSAGE_WAIT_DATA.write() else {
            return;
        };

        let Some(message_wait_data) = message_wait_data_guard.as_mut() else {
            return;
        };

        // Remove only the wait associated with this guard.
        // This prevents an old guard from removing a newer wait.
        let should_remove = message_wait_data.waiting_worker_registry.get(&self.worker_id).is_some_and(|wait_state| wait_state.message_instance_id == self.message_instance_id);

        if should_remove {
            message_wait_data.waiting_worker_registry.remove(&self.worker_id);
        }
    }
}

#[path = "../functions/register_worker_wait.rs"]
pub(crate) mod register_worker_wait;
