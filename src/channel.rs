use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot;

use crate::message::FrameworkMessage;

#[derive(Debug)]
pub(crate) enum LifecycleCommand {
    Init(oneshot::Sender<()>),
    Close(oneshot::Sender<()>),
}

/// Channel used to send lifecycle commands to a module.
pub(crate) type LifecycleSenderChannel = Sender<LifecycleCommand>;

/// Channel used to send business messages to a module.
pub(crate) type MessageSenderChannel = Sender<FrameworkMessage>;

/// Receiver used to receive lifecycle commands from MMF.
pub(crate) type LifecycleReceiverChannel = Receiver<LifecycleCommand>;

/// Receiver used to receive FrameworkMessages from MMF.
pub(crate) type MessageReceiverChannel = Receiver<FrameworkMessage>;
