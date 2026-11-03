use std::time::Duration;
use tokio::sync::oneshot;

use crate::channel::{WorkerChannel, WorkerCommand, send_worker_command};
use crate::mwf::{MWFError, WorkerError};
use crate::mwf_wrk::types::{EntityID, MessageID, MessageInheritanceContext, MessageInstanceID, WorkerID, WorkerMessage};

/// Send a message and wait until its callback has completed.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn wait_this_message<TMessageData>(sender_id: &EntityID, receiver_id: &WorkerID, message_id: &MessageID, message_instance_id: MessageInstanceID, message_data: &TMessageData, timeout_ms: Option<u64>, priority: bool, channel: WorkerChannel) -> Result<(), MWFError>
where
    TMessageData: Clone + Send + Sync + 'static,
{
    // Messages waiting for a callback do not inherit a message flow.
    let framework_message = WorkerMessage::new(sender_id, receiver_id, message_id, message_instance_id, message_data, MessageInheritanceContext::default());

    let (reply_sender, reply_receiver) = oneshot::channel::<Result<(), WorkerError>>();

    // Submit the message and wait for the callback result.
    let wait_future = async {
        send_worker_command(sender_id, receiver_id, channel, WorkerCommand::MessageAndWaitCompletion { reply_sender, message: framework_message, priority }).await?;

        match reply_receiver.await {
            Ok(worker_result) => worker_result.map_err(MWFError::Worker_MessageError),
            Err(_) => Err(MWFError::Worker_ChannelClosed),
        }
    };

    // Apply the timeout to the entire operation, including channel admission.
    match timeout_ms {
        Some(timeout_ms) => tokio::time::timeout(Duration::from_millis(timeout_ms), wait_future).await.map_err(|_| MWFError::Operation_Timeout)?,
        None => wait_future.await,
    }
}
