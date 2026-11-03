use std::time::Duration;
use tokio::sync::oneshot;

use crate::channel::{WorkerChannel, WorkerCommand, send_worker_command};
use crate::mwf::MWFError;
use crate::mwf_wrk::types::{EntityID, MessageFlowID, MessageID, MessageInheritanceContext, MessageInstanceID, WorkerID, WorkerMessage};

use crate::wait_registry::register_message_flow;

/// Send a message and wait until its entire message flow has completed.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn wait_message_flow<TMessageData>(
    sender_id: &EntityID,
    receiver_id: &WorkerID,
    message_id: &MessageID,
    message_instance_id: MessageInstanceID,
    message_flow_id: MessageFlowID,
    message_data: &TMessageData,
    timeout_ms: Option<u64>,
    priority: bool,
    channel: WorkerChannel,
) -> Result<(), MWFError>
where
    TMessageData: Clone + Send + Sync + 'static,
{
    // Register the new flow before submitting its initial message.
    let (flow_reply_sender, flow_reply_receiver) = oneshot::channel::<Result<(), MWFError>>();

    let mut flow_registration_guard = register_message_flow(message_flow_id, receiver_id.clone(), flow_reply_sender)?;

    // Associate the new flow with its initial message.
    let inheritance_context = MessageInheritanceContext { message_flow_id: Some(message_flow_id) };

    let framework_message = WorkerMessage::new(sender_id, receiver_id, message_id, message_instance_id, message_data, inheritance_context);

    // Submit the initial message and wait for explicit flow completion.
    let wait_future = async {
        send_worker_command(sender_id, receiver_id, channel, WorkerCommand::Message { message: framework_message, priority, unique: false }).await?;

        // Preserve the registration after successful submission.
        flow_registration_guard.disarm();

        match flow_reply_receiver.await {
            Ok(worker_result) => worker_result,
            Err(_) => Err(MWFError::Worker_ChannelClosed),
        }
    };

    // Apply the timeout to the entire operation, including channel admission.
    match timeout_ms {
        Some(timeout_ms) => tokio::time::timeout(Duration::from_millis(timeout_ms), wait_future).await.map_err(|_| MWFError::Operation_Timeout)?,
        None => wait_future.await,
    }
}
