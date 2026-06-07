//! MMF communication channels and commands.
//!
//! This module defines the channel types used to communicate
//! with ModuleTasks.
//!
//! All lifecycle operations (like Init or Close) and business 
//! messages are serialized through a single FIFO queue using 
//! ModuleCommand.
//! 
//! This guarantees that commands are processed by a module in the
//! same order they are accepted by MMF.

use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot;

use crate::message::FrameworkMessage;

/// Command processed by a ModuleTask.
///
/// All module events are transported through the same channel:
/// - Initialization requests
/// - Business messages
/// - Shutdown requests
///
/// Using a single command queue guarantees FIFO processing and
/// prevents lifecycle commands (like Init or Close) from overtaking 
/// previously accepted messages.
#[derive(Debug)]
pub(crate) enum ModuleCommand {
    Init(oneshot::Sender<()>),
    Message(FrameworkMessage),
    Close(oneshot::Sender<()>),
}

pub(crate) type ModuleSenderChannel = Sender<ModuleCommand>;

pub(crate) type ModuleReceiverChannel = Receiver<ModuleCommand>;
 
