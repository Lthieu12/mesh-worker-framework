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

use crate::channel::{LifecycleCommand, LifecycleReceiverChannel, MessageReceiverChannel};
use crate::module::Module;

/// Spawn a Tokio task that takes ownership of the module instance.
///
/// After this point:
/// - MMF owns the Senders and the JoinHandle.
/// - ModuleTask owns the module instance and the Receivers.
pub(crate) fn spawn_module_task<T>(module: T, lifecycle_receiver: LifecycleReceiverChannel, message_receiver: MessageReceiverChannel) -> JoinHandle<()>
where T: Module + Send + 'static{
   
    tokio::spawn(async move {
        // ModuleTask owns the module instance.
        //
        // All lifecycle callbacks and message callbacks are executed
        // inside this Tokio task.
        let mut module = module;

        let mut lifecycle_receiver = lifecycle_receiver;
        let _message_receiver = message_receiver;

        // Wait for lifecycle commands sent by MMF.
        //
        // The loop terminates when:
        // - A Close command is received.
        // - All LifecycleSenderChannels are dropped.
        while let Some(command) =
        lifecycle_receiver.recv().await
        {
            match command {

                // Execute module initialization.
                //
                // MMF is blocked waiting on the reply_receiver created
                // in init_module().
                //
                // The reply is sent only after on_init() completes,
                // making init_module() behave like a synchronous operation.
                LifecycleCommand::Init(reply_sender) => {
                    module.on_init().await;
                    let _ = reply_sender.send(());
                }

                // Execute module shutdown.
                //
                // MMF is blocked waiting for confirmation that on_close()
                // has completed.
                //
                // After the reply is sent, the ModuleTask exits its event
                // loop and the Tokio task terminates.
                LifecycleCommand::Close(reply_sender) => {
                    module.on_close().await;
                    let _ = reply_sender.send(());
                    break;
                }
            }
        }
    })
}