use super::super::MWFWrk;
use crate::channel::WorkerChannel;
use crate::channel::WorkerCommand;
use crate::channel::send_worker_command;
use crate::mwf::MWF;
use crate::mwf::MWFError;
use crate::mwf_wrk::types::EntityID;
use crate::mwf_wrk::types::MessageID;
use crate::mwf_wrk::types::WorkerID;
use crate::mwf_wrk::types::WorkerMessage;

impl MWFWrk {
    /// Send a message through the specified worker channel.
    pub(crate) async fn send_message_internal<TMessageID, TMessageData>(sender_id: &EntityID, receiver_id: &WorkerID, message_id: &TMessageID, message_data: &TMessageData, channel: WorkerChannel) -> Result<(), MWFError>
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

        // Determine the message priority and retrieve its sender channel.
        let priority = match &channel {
            WorkerChannel::Normal => false,
            WorkerChannel::Priority => true,
        };

        // Generate a unique identifier for this message instance.
        let message_instance_id = Self::generate_message_instance_id()?;

        // Inherit the current message flow only when sending to the same worker.
        let inheritance_context = Self::get_message_inheritance_context_for_receiver(receiver_id);

        let framework_message = WorkerMessage::new(sender_id, receiver_id, &message_id, message_instance_id, message_data, inheritance_context);

        // Send the message to the target worker.
        send_worker_command(sender_id, receiver_id, channel, WorkerCommand::Message { message: framework_message, priority, unique: false }).await
    }
}
