use crate::mwf_wrk::types::{EntityID, MessageData, MessageID, MessageInstanceID, WorkerID};

#[derive(Debug)]
/// Message delivered to a worker through MWF.
pub struct WorkerMessage {
    /// Identifier of the sender entity.
    pub sender_id: EntityID,

    /// Identifier of the receiver worker.
    pub receiver_id: WorkerID,

    /// Functional identifier of the message.
    pub message_id: MessageID,

    /// Unique identifier of this message instance.
    pub message_instance_id: MessageInstanceID,

    /// Data associated with the message.
    pub message_data: MessageData,
}

impl WorkerMessage {
    /// Create a new worker message.
    pub(crate) fn new<TMessageData>(sender_id: &EntityID, receiver_id: &WorkerID, message_id: &MessageID, message_instance_id: MessageInstanceID, message_data: &TMessageData) -> Self
    where
        TMessageData: Clone + Send + Sync + 'static,
    {
        Self { sender_id: sender_id.clone(), receiver_id: receiver_id.clone(), message_id: message_id.clone(), message_instance_id, message_data: MessageData::new(message_data) }
    }
}
