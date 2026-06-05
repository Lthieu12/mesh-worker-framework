use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::Receiver;

use crate::message::FrameworkMessage;

#[derive(Debug)]
pub(crate) enum LifecycleCommand {
    Init,
    Close,
}

/// Channel used to send lifecycle commands to a module.
pub(crate) type LifecycleSenderChannel = Sender<LifecycleCommand>;

/// Channel used to send business messages to a module.
pub(crate) type MessageSenderChannel = Sender<FrameworkMessage>;

/// Receiver used to receive lifecycle commands from MMF.
pub(crate) type LifecycleReceiverChannel = Receiver<LifecycleCommand>;

/// Receiver used to receive FrameworkMessages from MMF.
pub(crate) type MessageReceiverChannel = Receiver<FrameworkMessage>;
