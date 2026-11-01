use super::super::MWFWrk;
use crate::channel::WorkerChannel;
use crate::framework_error::MWFError;
use crate::message::MessageID;
use crate::message::MessageWaitMode;
use crate::worker::EntityID;
use crate::worker::WorkerID;

impl MWFWrk {
    /// Send a priority message and wait for the requested completion mode.
    ///
    /// Priority messages are processed before pending normal messages.
    ///
    /// `message_id` may be any type implementing `Into<MessageID>`.
    ///
    /// The completion mode determines what the caller waits for:
    /// - `WaitThisMessage`: completion of the receiver's on_message() callback.
    /// - `WaitThisMessageFlow`: explicit completion of the message flow through message_flow_end().
    ///
    /// The timeout covers the entire operation, including waiting for channel capacity
    /// and waiting for the requested completion.
    ///
    /// A timeout does not cancel a message that has already been accepted by the worker.
    /// The worker continues processing it.
    ///
    /// For WaitThisMessageFlow, a timeout after message admission does not cancel the
    /// MessageFlow. It remains active until message_flow_end() is called or the WorkerTask
    /// terminates.
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    ///
    /// Returns:
    /// - Ok(()) if the requested completion succeeds.
    /// - An MWFError if the callback or message flow returns an error, the worker terminates,
    ///   or the operation times out.
    pub async fn send_priority_message_and_wait<TMessageID, TMessageData>(sender_id: &EntityID, receiver_id: &WorkerID, message_id: &TMessageID, wait_mode: MessageWaitMode, message_data: &TMessageData, timeout_ms: Option<u64>) -> Result<(), MWFError>
    where
        TMessageID: Into<MessageID> + Clone,
        TMessageData: Clone + Send + Sync + 'static,
    {
        Self::send_message_and_wait_internal(sender_id, receiver_id, message_id, wait_mode, message_data, timeout_ms, WorkerChannel::Priority).await
    }
}
