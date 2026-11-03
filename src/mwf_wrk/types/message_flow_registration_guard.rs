use crate::mwf::MWF_MESSAGE_WAIT_DATA;
use crate::mwf_wrk::types::{MessageFlowID, WorkerID};

/// Automatically removes a newly registered flow if message submission fails.
#[derive(Debug)]
pub(crate) struct MessageFlowRegistrationGuard {
    /// Identifier of the registered message flow.
    message_flow_id: MessageFlowID,

    /// Worker that owns this message flow.
    worker_id: WorkerID,

    /// Indicates whether the registration still needs cleanup.
    armed: bool,
}

impl MessageFlowRegistrationGuard {
    /// Preserve the registration after the initial message has been submitted.
    pub(crate) fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for MessageFlowRegistrationGuard {
    fn drop(&mut self) {
        // Cleanup is needed only while submission has not been confirmed.
        if !self.armed {
            return;
        }

        // Do not panic during cleanup if the registry lock is poisoned.
        let Ok(mut message_wait_data_guard) = MWF_MESSAGE_WAIT_DATA.write() else {
            return;
        };

        let Some(message_wait_data) = message_wait_data_guard.as_mut() else {
            return;
        };

        // Remove only the flow registered by this guard and owned by this worker.
        let should_remove = message_wait_data.message_flow_registry.get(&self.message_flow_id).is_some_and(|flow_state| flow_state.worker_id == self.worker_id);

        if should_remove {
            message_wait_data.message_flow_registry.remove(&self.message_flow_id);
        }
    }
}

#[path = "../functions/register_message_flow.rs"]
pub(crate) mod register_message_flow;
