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
//! - Execute the module callbacks.
//! - Terminate the module task when requested.
//! - Process all module commands sequentially through a
//!   single FIFO queue.
//!
//! MMF never directly calls on_init(), on_message() or on_close().
//! These callbacks are executed inside the module task because
//! the module instance is owned by Tokio after spawn_module().

use tokio::task::JoinHandle;

use crate::channel::{ModuleCommand, ModuleReceiverChannel};
use crate::module::{Module, ModuleID, EntityID};

/// Spawn a Tokio task that takes ownership of the module instance.
///
/// After this point:
/// - MMF owns the Senders and the JoinHandle.
/// - ModuleTask owns the module instance and the Receivers.
pub(crate) fn spawn_module_task<T>(module: T, module_receiver: ModuleReceiverChannel, module_id:ModuleID, father_id:EntityID) -> JoinHandle<()>
where T: Module + Send + 'static{
   
    tokio::spawn(async move {
        // ModuleTask owns the module instance.
        //
        // All module callbacks are executed
        // inside this Tokio task.
        let mut module = module;
        let mut module_receiver = module_receiver;

        let module_id = module_id;
        let father_id = father_id;

        // Wait for module commands sent by MMF.
        //
        // The loop terminates when:
        // - A Close command is received.
        // - All ModuleSenderChannels are dropped.
        while let Some(command) =
        module_receiver.recv().await
        {
            match command {

                // Execute module initialization.
                //
                // MMF is blocked waiting on the reply_receiver created
                // in init_module().
                //
                // The reply is sent only after on_init() completes,
                // making init_module() behave like a synchronous operation.
                ModuleCommand::Init(reply_sender) => {
                    module.on_init(&module_id, &father_id).await;
                    let _ = reply_sender.send(());
                }

                // Execute a business message.
                //
                // Messages are processed through the same FIFO queue
                // as lifecycle commands (like Init or Close).
                //
                // This guarantees that a message accepted before a
                // Close command is always processed before module
                // shutdown begins.
                ModuleCommand::Message(message) => {
                    module.on_message(message).await;
                }

                // Execute module shutdown.
                //
                // MMF is blocked waiting for confirmation that on_close()
                // has completed.
                //
                // After the reply is sent, the ModuleTask exits its event
                // loop and the Tokio task terminates.
                ModuleCommand::Close(reply_sender) => {
                    module.on_close().await;
                    let _ = reply_sender.send(());
                    break;
                }
            }
        }
    })
    
}