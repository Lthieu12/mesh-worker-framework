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

use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot;

use crate::message::FrameworkMessage;
use crate::framework_error::ModuleError;

/// Command processed by a ModuleTask.
#[derive(Debug)]
pub(crate) enum ModuleCommand {
    Init,
    Message(FrameworkMessage),
    MessageAndWaitReply(oneshot::Sender<Result<(), ModuleError>>, FrameworkMessage),
    Close,
}

pub(crate) type ModuleNormalSenderChannel = Sender<ModuleCommand>;
pub(crate) type ModuleNormalReceiverChannel = Receiver<ModuleCommand>;

pub(crate) type ModulePrioritySenderChannel = Sender<ModuleCommand>;
pub(crate) type ModulePriorityReceiverChannel = Receiver<ModuleCommand>;
 
