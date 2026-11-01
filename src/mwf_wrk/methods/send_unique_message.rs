use super::super::MWFWrk;
use crate::channel::WorkerChannel;
use crate::channel::WorkerCommand;
use crate::channel::send_worker_command;
use crate::framework_error::MWFError;
use crate::message::MessageID;
use crate::message::WorkerMessage;
use crate::mwf::MWF;
use crate::worker::EntityID;
use crate::worker::WorkerID;

impl MWFWrk {
    /// Send a unique message to a worker.
    ///
    /// At most one unique message with the same MessageID
    /// is retained in the worker's pending command queue.
    ///
    /// Sending a new unique message replaces the pending one while
    /// preserving its FIFO position.
    ///
    /// `message_id` may be any type implementing `Into<MessageID>`.
    pub async fn send_unique_message<TMessageID, TMessageData>(sender_id: &EntityID, receiver_id: &WorkerID, message_id: &TMessageID, message_data: &TMessageData) -> Result<(), MWFError>
    where
        TMessageID: Into<MessageID> + Clone,
        TMessageData: Clone + Send + Sync + 'static,
    {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        // Validate the entity identifier.
        if !sender_id.is_valid() {
            return Err(MWFError::Entity_Invalid { entity_id: sender_id.clone() });
        }
        // Convert the message identifier.
        let message_id = message_id.clone().into();
        if !message_id.is_valid() {
            return Err(MWFError::Message_Invalid { message_id });
        }

        // Generate the unique id and build the message
        let message_instance_id = MWFWrk::generate_message_instance_id()?;

        // Inherit the current message flow only when sending a message to this same worker.
        let inheritance_context = MWFWrk::get_message_inheritance_context_for_receiver(receiver_id);

        let framework_message = WorkerMessage::new(sender_id, receiver_id, &message_id, message_instance_id, message_data, inheritance_context);

        // Send the unique message to the target worker.
        send_worker_command(sender_id, receiver_id, WorkerChannel::Normal, WorkerCommand::Message { message: framework_message, priority: false, unique: true }).await
    }
}
