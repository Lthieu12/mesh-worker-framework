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

use crate::mmf::{MMF_DATA, MMF_MODULE_INIT_NOTIFY, MMF_MODULE_CLOSE_NOTIFY};
use crate::channel::{ModuleCommand, ModuleReceiverChannel};
use crate::module::{Module, ModuleID, EntityID, ModuleState};

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
                // MMF has already changed the module state from
                // Created to Initializing before sending this command.
                //
                // ModuleTask is responsible for marking the module
                // as Initialized only after on_init() completes.
                ModuleCommand::Init => {

                    module.on_init(&module_id, &father_id).await;

                    // Publish successful initialization.
                    //
                    // The module is now fully initialized and can
                    // receive business messages.
                    let mut mmf_data_guard = MMF_DATA.write().unwrap();
                    let mmf_data = mmf_data_guard.as_mut().unwrap();
                    mmf_data.registry.set_module_state(&module_id, ModuleState::Initialized).unwrap();
                    drop(mmf_data_guard);

                    // Wake up all waiters interested in module
                    // initialization events.
                    MMF_MODULE_INIT_NOTIFY.notify_waiters();
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

                // Execute a business message.
                //
                // MMF is blocked waiting for confirmation that on_message()
                // has completed.
                //
                // Messages are processed through the same FIFO queue
                // as lifecycle commands (like Init or Close).
                //
                // This guarantees that a message accepted before a
                // Close command is always processed before module
                // shutdown begins.
                ModuleCommand::MessageAndWaitReply(reply_sender, message) => {
                    module.on_message(message).await;
                    let _ = reply_sender.send(());
                }

                // Execute module shutdown.
                //
                // MMF has already changed the module state from
                // Initialized to Closing before sending this command.
                //
                // ModuleTask is responsible for removing the module
                // from the Registry only after on_close() completes.
                ModuleCommand::Close => {

                    module.on_close().await;

                    // Mark the module as Closed.
                    //
                    // Closed indicates that the module lifecycle
                    // completed successfully.
                    let mut mmf_data_guard = MMF_DATA.write().unwrap();
                    let mmf_data = mmf_data_guard.as_mut().unwrap();
                    mmf_data.registry.set_module_state(&module_id, ModuleState::Closed).unwrap();

                    // Remove the module from the Registry.
                    //
                    // Once removed, the module is no longer visible
                    // to MMF and cannot receive new commands.
                    mmf_data.registry.remove_module(&module_id).unwrap();
                    drop(mmf_data_guard);

                    // Wake up all waiters interested in module
                    // shutdown events.
                    MMF_MODULE_CLOSE_NOTIFY.notify_waiters();

                    // Terminate the Tokio task associated with
                    // this module.
                    break;
                }
            }
        }
    })
    
}