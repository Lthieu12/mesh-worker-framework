//! MWF communication channels and commands.
//!
//! This file defines the channel types used to communicate
//! with WorkerTasks.
//!
//! All worker events are transported through WorkerCommand.
//!
//! Normal commands are processed FIFO.
//!
//! Priority commands are received through a dedicated channel
//! and processed before normal commands using a LIFO policy.
//!
//! This guarantees that commands are processed by a worker in the
//! same order they are accepted by MWF.

use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;

use crate::framework_error::{MWFError, MWFPanic, WorkerError};
use crate::message::{MessageData, MessageID, WorkerMessage};
use crate::mwf::MWF_WRK_DATA;
use crate::worker::{EntityID, WorkerID, WorkerState};
use crate::worker_registry::Registry;

pub(crate) type WorkerSenderChannel = Sender<WorkerCommand>;
pub(crate) type WorkerReceiverChannel = Receiver<WorkerCommand>;

pub(crate) type WorkerNormalSenderChannel = WorkerSenderChannel;
pub(crate) type WorkerNormalReceiverChannel = WorkerReceiverChannel;
pub(crate) type WorkerPrioritySenderChannel = WorkerSenderChannel;
pub(crate) type WorkerPriorityReceiverChannel = WorkerReceiverChannel;

pub(crate) enum WorkerChannel {
    Normal,
    Priority,
}

/// Command processed by a WorkerTask.
#[derive(Debug)]
pub(crate) enum WorkerCommand {
    Init { init_data: MessageData },
    Message { message: WorkerMessage, priority: bool, unique: bool },
    MessageAndWaitCompletion { reply_sender: oneshot::Sender<Result<(), WorkerError>>, message: WorkerMessage, priority: bool },
    Close { close_data: MessageData },
}

impl WorkerCommand {
    /// Return whether this command is unique.
    pub(crate) fn is_unique_message(&self) -> bool {
        match self {
            WorkerCommand::Message { unique, .. } => *unique,

            _ => false,
        }
    }

    /// Return the MessageID carried by this command.
    pub(crate) fn message_id(&self) -> Option<&MessageID> {
        match self {
            WorkerCommand::Message { message, .. } | WorkerCommand::MessageAndWaitCompletion { message, .. } => Some(&message.message_id),
            _ => None,
        }
    }
}

/// Validate that both the sender and receiver workers are initialized.
fn validate_sender_and_receiver(worker_registry: &Registry, sender_id: &EntityID, receiver_id: &WorkerID) -> Result<(), MWFError> {
    // External senders are not managed by MWF.
    if let EntityID::Worker(sender_worker_id) = sender_id {
        let worker_state = worker_registry.get_worker_state(sender_worker_id)?;

        if worker_state != WorkerState::Initialized {
            return Err(MWFError::Worker_NotInitialized);
        }
    }

    // Messages can only be sent to initialized workers.
    let worker_state = worker_registry.get_worker_state(receiver_id)?;

    if worker_state != WorkerState::Initialized {
        return Err(MWFError::Worker_NotInitialized);
    }

    Ok(())
}

/// Retrieve the sender channel associated with a worker.
///
/// The sender and receiver states are validated before the
/// channel is returned.
pub(crate) fn get_sender_channel(sender_id: &EntityID, receiver_id: &WorkerID, channel: WorkerChannel) -> Result<WorkerSenderChannel, MWFError> {
    let mwf_data_guard = MWF_WRK_DATA.read().expect(MWFPanic::MWFWrkData_ReadLockPoisoned.message());
    let mwf_data = mwf_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;

    validate_sender_and_receiver(&mwf_data.worker_registry, sender_id, receiver_id)?;

    // Retrieve the communication channel associated with the receiver.
    match channel {
        WorkerChannel::Normal => mwf_data.worker_registry.get_worker_normal_channel(receiver_id),
        WorkerChannel::Priority => mwf_data.worker_registry.get_worker_priority_channel(receiver_id),
    }
}

/// Send a worker command while synchronizing message admission with worker shutdown.
///
/// The channel capacity is reserved before acquiring MWF_WRK_DATA.
/// Worker states are checked again immediately before inserting the command.
pub(crate) async fn send_worker_command(sender_id: &EntityID, receiver_id: &WorkerID, channel: WorkerChannel, command: WorkerCommand) -> Result<(), MWFError> {
    // Validate the workers and retrieve the sender channel.
    let sender_channel = get_sender_channel(sender_id, receiver_id, channel)?;

    // Reserve channel capacity without holding the registry lock.
    let permit = sender_channel.reserve().await.map_err(|_| MWFError::Worker_ChannelClosed)?;

    // Revalidate worker states while synchronizing with close_worker().
    let mwf_data_guard = MWF_WRK_DATA.read().expect(MWFPanic::MWFWrkData_ReadLockPoisoned.message());
    let mwf_data = mwf_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;

    validate_sender_and_receiver(&mwf_data.worker_registry, sender_id, receiver_id)?;

    // Insert the command before releasing the registry lock.
    permit.send(command);

    Ok(())
}
