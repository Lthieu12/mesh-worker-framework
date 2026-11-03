use super::MessageFlowID;

/// Context inherited by messages sent during message processing.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct MessageInheritanceContext {
    /// Flow associated with the current message, if any.
    pub(crate) message_flow_id: Option<MessageFlowID>,
}
