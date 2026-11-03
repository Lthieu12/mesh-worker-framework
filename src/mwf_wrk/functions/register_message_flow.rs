use tokio::sync::oneshot;

use crate::mwf::MWF_MESSAGE_WAIT_DATA;
use crate::mwf::{MWFError, MWFPanic};
use crate::mwf_wrk::types::{MessageFlowID, MessageFlowRegistrationGuard, MessageFlowState, WorkerID};

/// Register a new message flow before submitting its initial message.
pub(crate) fn register_message_flow(message_flow_id: MessageFlowID, worker_id: WorkerID, reply_sender: oneshot::Sender<Result<(), MWFError>>) -> Result<MessageFlowRegistrationGuard, MWFError> {
    let mut message_wait_data_guard = MWF_MESSAGE_WAIT_DATA.write().expect(MWFPanic::MWFMessageWaitData_WriteLockPoisoned.message());

    let message_wait_data = message_wait_data_guard.as_mut().ok_or(MWFError::Framework_NotInitialized)?;

    // A generated flow ID must never replace an existing flow.
    if message_wait_data.message_flow_registry.contains_key(&message_flow_id) {
        return Err(MWFError::MessageFlow_RegistryInconsistent);
    }

    message_wait_data.message_flow_registry.insert(message_flow_id, MessageFlowState { worker_id: worker_id.clone(), reply_sender });

    drop(message_wait_data_guard);

    Ok(MessageFlowRegistrationGuard { message_flow_id, worker_id, armed: true })
}
