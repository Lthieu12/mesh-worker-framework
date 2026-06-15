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
//! - Process all module commands sequentially.
//!
//! Normal commands are processed FIFO.
//! Priority commands are processed before normal commands
//! using a LIFO policy.
//!
//! MMF never directly calls on_init(), on_message() or on_close().
//! These callbacks are executed inside the module task because
//! the module instance is owned by Tokio after spawn_module().

use tokio::task::JoinHandle;

use crate::mmf::{MMF_DATA, MMF_MODULE_INIT_NOTIFY, MMF_MODULE_CLOSE_NOTIFY};
use crate::channel::{ModuleCommand, ModuleNormalReceiverChannel, ModulePriorityReceiverChannel};
use crate::module::{Module, ModuleID, EntityID, ModuleState};

/// Spawn a Tokio task that takes ownership of the module instance.
///
/// After this point:
/// - MMF owns the Senders and the JoinHandle.
/// - ModuleTask owns the module instance and the Receivers.
pub(crate) fn spawn_module_task<T>(module: T, normal_receiver_channel: ModuleNormalReceiverChannel, priority_receiver_channel: ModulePriorityReceiverChannel, module_id:ModuleID, father_id:EntityID) -> JoinHandle<()>
where T: Module + Send + 'static{
   
    tokio::spawn(async move {
        // ModuleTask owns the module instance.
        //
        // All module callbacks are executed
        // inside this Tokio task.
        let mut module = module;
        let mut normal_receiver_channel = normal_receiver_channel;
        let mut priority_receiver_channel = priority_receiver_channel;

        let module_id = module_id;
        let father_id = father_id;

        // Local LIFO stack used to store priority commands.
        //
        // Priority commands are first received through a Tokio FIFO
        // channel, then moved into this stack.
        //
        // This reproduces the behavior:
        //
        // send_priority_message(P1)
        // send_priority_message(P2)
        // send_priority_message(P3)
        //
        // Processing order:
        // P3 -> P2 -> P1
        let mut priority_stack: Vec<ModuleCommand> = Vec::new();

        loop {

            // Drain all currently pending priority commands.
            //
            // Tokio channels are FIFO. Moving all available priority
            // commands into a local stack allows ModuleTask to process
            // them in LIFO order.
            while let Ok(command) = priority_receiver_channel.try_recv() {
                priority_stack.push(command);
            }

            // Priority commands always take precedence over normal
            // commands.
            //
            // If the stack is not empty, process the most recently
            // received priority command immediately.
            let command =
                if let Some(command) = priority_stack.pop() {

                    command
                } 
                else {

                    // No priority command is currently pending.
                    //
                    // Wait for the next command from either channel.
                    //
                    // Tokio wakes this task automatically when a message
                    // arrives on the normal or priority channel.
                    //
                    // Receiver channels are expected to remain valid
                    // until the module task terminates.
                    //
                    // A closed channel indicates an internal framework error.
                    tokio::select! {
                        command = priority_receiver_channel.recv() => {
                            command.unwrap()
                        }
                        command = normal_receiver_channel.recv() => {
                            command.unwrap()
                        }
                    }
                };          

            match command {

                // Execute module initialization.
                //
                // MMF has already changed the module state from
                // Created to Initializing before sending this command.
                //
                // ModuleTask is responsible for marking the module
                // as Initialized only after on_init() completes.
                ModuleCommand::Init => {

                    let error_init = module.on_init(&module_id, &father_id).await;

                    // Publish successful initialization.
                    //
                    // The module is now fully initialized and can
                    // receive business messages.
                    let mut mmf_data_guard = MMF_DATA.write().unwrap();
                    let mmf_data = mmf_data_guard.as_mut().unwrap();
                    match error_init {
                        Ok(()) => {

                            mmf_data.registry.set_module_state(&module_id, ModuleState::Initialized).unwrap();
                        }
                        Err(_) => {

                            mmf_data.registry.set_module_state(&module_id, ModuleState::InitFailed).unwrap();
                        }

                    }
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
                    let _ = module.on_message(message).await;
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
 
                        let result = module.on_message(message).await;
                        let _ = reply_sender.send(result);
                    }

                // Execute module shutdown.
                //
                // MMF has already changed the module state from
                // Initialized to Closing before sending this command.
                //
                // ModuleTask is responsible for removing the module
                // from the Registry only after on_close() completes.
                ModuleCommand::Close => {

                    let error_close = module.on_close().await;

                    // Mark the module as Closed.
                    //
                    // Closed indicates that the module lifecycle
                    // completed successfully.
                    let mut mmf_data_guard = MMF_DATA.write().unwrap();
                    let mmf_data = mmf_data_guard.as_mut().unwrap();
                    match error_close {
                        Ok(()) => {

                            mmf_data.registry.set_module_state(&module_id, ModuleState::Closed).unwrap();
                            // Remove the module from the Registry.
                            //
                            // Once removed, the module is no longer visible
                            // to MMF and cannot receive new commands.
                            mmf_data.registry.remove_module(&module_id).unwrap();
                        }
                        Err(_) => {

                            mmf_data.registry.set_module_state(&module_id, ModuleState::CloseFailed).unwrap();
                        }

                    }

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