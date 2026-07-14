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

use crate::framework_error::ModuleError;
use crate::message::ModuleMessage;

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
    pub(crate) fn is_unique(&self) -> bool {
        match self {
            ModuleCommand::Message { unique, .. } => *unique,

            _ => false,
        }
    }
}

pub(crate) type ModuleNormalSenderChannel = Sender<ModuleCommand>;
pub(crate) type ModuleNormalReceiverChannel = Receiver<ModuleCommand>;

pub(crate) type ModulePrioritySenderChannel = Sender<ModuleCommand>;
pub(crate) type ModulePriorityReceiverChannel = Receiver<ModuleCommand>;
