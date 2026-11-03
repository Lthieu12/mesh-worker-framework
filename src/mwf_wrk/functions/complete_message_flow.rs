use crate::mwf::MWF_MESSAGE_WAIT_DATA;
use crate::mwf::{MWFError, MWFPanic, WorkerError};
use crate::mwf_wrk::types::{MessageFlowID, MessageFlowState, WorkerID};

/// Complete a message flow and notify its caller.
pub(crate) fn complete_message_flow(message_flow_id: MessageFlowID, worker_id: &WorkerID, result: Result<(), WorkerError>) -> Result<(), MWFError> {
    let flow_state = take_message_flow(message_flow_id, worker_id)?;

    // The caller may already have stopped waiting, for example because of a timeout.
    let _ = flow_state.reply_sender.send(result.map_err(MWFError::Worker_MessageError));

    Ok(())
}

/// Remove a message flow and return its state.
///
/// Only the worker that owns the flow may remove it.
fn take_message_flow(message_flow_id: MessageFlowID, worker_id: &WorkerID) -> Result<MessageFlowState, MWFError> {
    let mut message_wait_data_guard = MWF_MESSAGE_WAIT_DATA.write().expect(MWFPanic::MWFMessageWaitData_WriteLockPoisoned.message());

    let message_wait_data = message_wait_data_guard.as_mut().ok_or(MWFError::Framework_NotInitialized)?;

    // Validate that the flow exists and belongs to the current worker.
    let flow_state = message_wait_data.message_flow_registry.get(&message_flow_id).ok_or(MWFError::MessageFlow_NotActive)?;

    if &flow_state.worker_id != worker_id {
        return Err(MWFError::MessageFlow_NotActive);
    }

    // Remove the entry while still holding the write lock.
    message_wait_data.message_flow_registry.remove(&message_flow_id).ok_or(MWFError::MessageFlow_NotActive)
}
