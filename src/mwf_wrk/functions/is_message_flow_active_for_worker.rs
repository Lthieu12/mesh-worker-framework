use crate::mwf::MWF_MESSAGE_WAIT_DATA;
use crate::mwf::MWFPanic;
use crate::mwf_wrk::types::{MessageFlowID, WorkerID};

/// Returns whether a message flow is still active for the specified worker.
///
/// A missing message flow is treated as inactive so stale message contexts
/// can be safely ignored.
pub(crate) fn is_message_flow_active_for_worker(message_flow_id: MessageFlowID, worker_id: &WorkerID) -> bool {
    let message_wait_data_guard = MWF_MESSAGE_WAIT_DATA.read().expect(MWFPanic::MWFMessageWaitData_ReadLockPoisoned.message());

    message_wait_data_guard.as_ref().and_then(|message_wait_data| message_wait_data.message_flow_registry.get(&message_flow_id)).is_some_and(|message_flow_state| &message_flow_state.worker_id == worker_id)
}
