//! Tokio runtime for MMF modules.
//!
//! This module owns the Tokio task associated with a module instance.
//!
//! After a module is spawned:
//! - MMF owns the Senders and the JoinHandle.
//! - ModuleTask owns the module instance and the Receivers.
//! 
//! Responsibilities:
//! - Own the module instance.
//! - Receive commands and messages sent by MMF.
//! - Execute the module lifecycle callbacks.
//! - Terminate the module task when requested.
//!
//! MMF never directly calls on_init(), on_message() or on_close().
//! These callbacks are executed inside the module task because
//! the module instance is owned by Tokio after spawn_module().

use tokio::task::JoinHandle;

use crate::{channel::{LifecycleReceiverChannel, MessageReceiverChannel}, module::Module};

pub(crate) fn spawn_module_task<T>(module: T, lifecycle_receiver: LifecycleReceiverChannel, message_receiver: MessageReceiverChannel) -> JoinHandle<()>
where T: Module + Send + 'static{
   
    tokio::spawn(async move {

        let mut _module = module;

        let _lifecycle_receiver = lifecycle_receiver;
        let _message_receiver = message_receiver;

    })
}