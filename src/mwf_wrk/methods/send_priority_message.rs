use super::super::MWFWrk;
use crate::channel::WorkerChannel;
use crate::framework_error::MWFError;
use crate::message::MessageID;
use crate::worker::EntityID;
use crate::worker::WorkerID;

impl MWFWrk {
    /// Send a priority message to a worker.
    ///
    /// Priority messages are always processed before pending
    /// normal messages.
    ///
    /// `message_id` may be any type implementing `Into<MessageID>`.
    pub async fn send_priority_message<TMessageID, TMessageData>(sender_id: &EntityID, receiver_id: &WorkerID, message_id: &TMessageID, message_data: &TMessageData) -> Result<(), MWFError>
    where
        TMessageID: Into<MessageID> + Clone,
        TMessageData: Clone + Send + Sync + 'static,
    {
        Self::send_message_internal(sender_id, receiver_id, message_id, message_data, WorkerChannel::Priority).await
    }
}
