use super::super::MWFWrk;
use crate::message::MessageInheritanceContext;
use crate::message_wait::is_message_flow_active_for_worker;
use crate::worker::WorkerID;
use crate::worker_task::current_message_inheritance_context;
use crate::worker_task::current_worker_id;

impl MWFWrk {
    /// Returns the current message flow context only for messages sent to the current worker.
    ///
    /// The message flow must still be active and owned by that worker.
    /// Messages sent to another worker never inherit the current message flow.
    pub(crate) fn get_message_inheritance_context_for_receiver(receiver_id: &WorkerID) -> MessageInheritanceContext {
        // Avoid registry access when no message flow is active in the current context.
        let inheritance_context = current_message_inheritance_context();
        let Some(message_flow_id) = inheritance_context.message_flow_id else {
            return MessageInheritanceContext::default();
        };

        // Message flow contexts are only propagated to the current worker itself.
        if current_worker_id().as_ref() != Some(receiver_id) {
            return MessageInheritanceContext::default();
        }

        // Ignore a message flow that has already been closed or belongs to another worker.
        if is_message_flow_active_for_worker(message_flow_id, receiver_id) { inheritance_context } else { MessageInheritanceContext::default() }
    }
}
