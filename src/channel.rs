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

use bytes::Bytes;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;

use crate::framework_error::{MWFError, MWFPanic, WorkerError};
use crate::message::{MessageID, WorkerMessage};
use crate::mwf::MWF_WRK_DATA;
use crate::worker::{EntityID, WorkerID, WorkerState};

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
    Init { lifecycle_data: Bytes },
    Message { message: WorkerMessage, priority: bool, unique: bool },
    MessageAndWaitCompletion { reply_sender: oneshot::Sender<Result<(), WorkerError>>, message: WorkerMessage, priority: bool },
    Close { lifecycle_data: Bytes },
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

/// Retrieve the sender channel associated with a worker.
///
/// The sender and receiver states are validated before the
/// channel is returned.
pub(crate) fn get_sender_channel(sender_id: &EntityID, receiver_id: &WorkerID, channel: WorkerChannel) -> Result<WorkerSenderChannel, MWFError> {
    let sender_channel: WorkerNormalSenderChannel = {
        // Read MWF data.
        //
        // send_message() only requires a read lock because it does
        // not modify the Registry or workerState.
        let mwf_data_guard = MWF_WRK_DATA.read().expect(MWFPanic::MWFWrkData_ReadLockPoisoned.message());

        let mwf_data = mwf_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;

        // Validate the sender.
        //
        // MWF workers must be initialized before they can
        // send messages through MWF.
        //
        // External senders are not validated.
        match sender_id {
            EntityID::Worker(worker_id) => {
                let worker_state = mwf_data.worker_registry.get_worker_state(worker_id)?;
                if worker_state != WorkerState::Initialized {
                    return Err(MWFError::Worker_NotInitialized);
                }
            }
            EntityID::External(_) => {}
        }

        // Validate the receiver worker state.
        //
        // Messages can only be sent to initialized workers.
        let worker_state = mwf_data.worker_registry.get_worker_state(receiver_id)?;
        if worker_state != WorkerState::Initialized {
            return Err(MWFError::Worker_NotInitialized);
        }

        // Retrieve the communication channel associated with
        // the receiver worker.
        //
        // State validation and channel retrieval are performed
        // under the same lock to avoid race conditions with
        // close_worker().
        match channel {
            WorkerChannel::Normal => mwf_data.worker_registry.get_worker_normal_channel(receiver_id),
            WorkerChannel::Priority => mwf_data.worker_registry.get_worker_priority_channel(receiver_id),
        }?
    };
    Ok(sender_channel)
}
