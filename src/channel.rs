//! MMF communication channels and commands.
//!
//! This module defines the channel types used to communicate
//! with ModuleTasks.
//!
//! All module events are transported through ModuleCommand.
//!
//! Normal commands are processed FIFO.
//!
//! Priority commands are received through a dedicated channel
//! and processed before normal commands using a LIFO policy.
//!
//! This guarantees that commands are processed by a module in the
//! same order they are accepted by MMF.

use bytes::Bytes;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;

use crate::mmf::MMF_MOD_DATA;
use crate::framework_error::{FrameworkError, ModuleError, MMFPanic};
use crate::message::{MessageID, ModuleMessage};
use crate::module::{EntityID, ModuleID, ModuleState};

pub(crate) type ModuleSenderChannel = Sender<ModuleCommand>;
pub(crate) type ModuleReceiverChannel = Receiver<ModuleCommand>;

pub(crate) type ModuleNormalSenderChannel = ModuleSenderChannel;
pub(crate) type ModuleNormalReceiverChannel = ModuleReceiverChannel;
pub(crate) type ModulePrioritySenderChannel = ModuleSenderChannel;
pub(crate) type ModulePriorityReceiverChannel = ModuleReceiverChannel;

pub(crate) enum ModuleChannel {
    Normal,
    Priority,
}

/// Command processed by a ModuleTask.
#[derive(Debug)]
pub(crate) enum ModuleCommand {
    Init { lifecycle_data: Bytes },
    Message { message: ModuleMessage, priority: bool, unique: bool },
    MessageAndWaitCompletion { reply_sender: oneshot::Sender<Result<(), ModuleError>>, message: ModuleMessage, priority: bool, unique: bool },
    Close { lifecycle_data: Bytes },
}

impl ModuleCommand {
    /// Return whether this command is unique.
    pub(crate) fn is_unique_message(&self) -> bool {
        match self {
            ModuleCommand::Message { unique, .. } => *unique,

            _ => false,
        }
    }

    /// Return the MessageID carried by this command.
    pub(crate) fn message_id(&self) -> Option<&MessageID> {
        match self {
            ModuleCommand::Message { message, .. } | ModuleCommand::MessageAndWaitCompletion { message, .. } => Some(&message.message_id),
            _ => None,
        }
    }
}

/// Retrieve the sender channel associated with a module.
///
/// The sender and receiver states are validated before the
/// channel is returned.
pub(crate) fn get_sender_channel(
    sender_id: &EntityID,
    receiver_id: &ModuleID,
    channel: ModuleChannel,
) -> Result<ModuleSenderChannel, FrameworkError> {

    let sender_channel: ModuleNormalSenderChannel = {
        // Read MMF data.
        //
        // send_message() only requires a read lock because it does
        // not modify the Registry or ModuleState.
        let mmf_data_guard = MMF_MOD_DATA.read().expect(MMFPanic::MMFModData_ReadLockPoisoned.message());

        let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;

        // Validate the sender.
        //
        // MMF modules must be initialized before they can
        // send messages through MMF.
        //
        // External senders are not validated.
        match sender_id {
            EntityID::Module(module_id) => {
                let module_state = mmf_data.module_registry.get_module_state(module_id)?;
                if module_state != ModuleState::Initialized {
                    return Err(FrameworkError::Module_NotInitialized);
                }
            }
            EntityID::External(_) => {}
        }

        // Validate the receiver module state.
        //
        // Messages can only be sent to initialized modules.
        let module_state = mmf_data.module_registry.get_module_state(receiver_id)?;
        if module_state != ModuleState::Initialized {
            return Err(FrameworkError::Module_NotInitialized);
        }

        // Retrieve the communication channel associated with
        // the receiver module.
        //
        // State validation and channel retrieval are performed
        // under the same lock to avoid race conditions with
        // close_module().
        match channel {
            ModuleChannel::Normal => {
                mmf_data.module_registry.get_module_normal_channel(receiver_id)
            }
            ModuleChannel::Priority => {
                mmf_data.module_registry.get_module_priority_channel(receiver_id)
            }
        }?
    };
    Ok(sender_channel)
}
