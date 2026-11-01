use super::super::MWFWrk;
use crate::channel::WorkerChannel;
use crate::framework_error::MWFError;
use crate::message::MessageFlowID;
use crate::message::MessageID;
use crate::message::MessageInstanceID;
use crate::message::MessageWaitMode;
use crate::message_wait::WorkerWaitGuard;
use crate::message_wait::WorkerWaitState;
use crate::message_wait::register_worker_wait;
use crate::message_wait::wait_message_flow;
use crate::message_wait::wait_this_message;
use crate::mwf::MWF;
use crate::mwf::MWF_WRK_MESSAGE_FLOW_COUNTER;
use crate::worker::EntityID;
use crate::worker::WorkerID;
use crate::worker_task::current_worker_id;
use std::sync::atomic::Ordering;
use std::time::Instant;

impl MWFWrk {
    /// Generate a unique MessageFlowID.
    fn generate_message_flow_id() -> Result<MessageFlowID, MWFError> {
        MWF_WRK_MESSAGE_FLOW_COUNTER.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current_id| current_id.checked_add(1)).map_err(|_| MWFError::MessageFlow_InstanceIDExhausted)
    }

    /// Register the current worker's active message wait, when applicable.
    ///
    /// External application tasks have no MWF worker execution context,
    /// so their waits are not added to the worker wait registry.
    fn register_current_worker_wait(receiver_id: &WorkerID, message_id: &MessageID, message_instance_id: MessageInstanceID, wait_mode: MessageWaitMode) -> Result<Option<WorkerWaitGuard>, MWFError> {
        // Identify the worker from the actual execution context.
        let Some(waiting_worker_id) = current_worker_id() else {
            return Ok(None);
        };

        // Register the wait and reject any operation that would create a deadlock.
        let worker_wait_guard = register_worker_wait(&waiting_worker_id, WorkerWaitState { receiver_id: receiver_id.clone(), message_id: message_id.clone(), message_instance_id, _wait_mode: wait_mode, _started_at: Instant::now() })?;

        Ok(Some(worker_wait_guard))
    }

    /// Send a message and wait for the requested completion mode.
    pub(crate) async fn send_message_and_wait_internal<TMessageID, TMessageData>(sender_id: &EntityID, receiver_id: &WorkerID, message_id: &TMessageID, wait_mode: MessageWaitMode, message_data: &TMessageData, timeout_ms: Option<u64>, channel: WorkerChannel) -> Result<(), MWFError>
    where
        TMessageID: Into<MessageID> + Clone,
        TMessageData: Clone + Send + Sync + 'static,
    {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        // Validate the sender entity identifier.
        if !sender_id.is_valid() {
            return Err(MWFError::Entity_Invalid { entity_id: sender_id.clone() });
        }

        // Validate the message identifier.
        let message_id = message_id.clone().into();
        if !message_id.is_valid() {
            return Err(MWFError::Message_Invalid { message_id });
        }

        // Resolve the channel and whether the message is a priority message.
        let priority = match &channel {
            WorkerChannel::Normal => false,
            WorkerChannel::Priority => true,
        };

        // Generate an identifier for this message instance.
        let message_instance_id = Self::generate_message_instance_id()?;

        // Register the current worker's wait to detect deadlocks.
        let _worker_wait_guard = Self::register_current_worker_wait(receiver_id, &message_id, message_instance_id, wait_mode)?;

        match wait_mode {
            MessageWaitMode::WaitThisMessage => wait_this_message(sender_id, receiver_id, &message_id, message_instance_id, message_data, timeout_ms, priority, channel).await,
            MessageWaitMode::WaitThisMessageFlow => {
                let message_flow_id = Self::generate_message_flow_id()?;

                wait_message_flow(sender_id, receiver_id, &message_id, message_instance_id, message_flow_id, message_data, timeout_ms, priority, channel).await
            }
        }
    }
}
