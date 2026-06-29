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
use std::time::Instant;
use std::panic::AssertUnwindSafe;
use futures::FutureExt;

use crate::mmf::{MMF_MOD_DATA, MMF_MOD_INIT_NOTIFY, MMF_MOD_CLOSE_NOTIFY};
use crate::channel::{ModuleCommand, ModuleNormalReceiverChannel, ModulePriorityReceiverChannel};
use crate::module::{Module, ModuleID, EntityID, ModuleState};
use crate::framework_error::MMFPanic;
use crate::MMFErr;
use crate::MMFLog;
use crate::mmf_log::{LogAction, LogStatus};


/// Spawn the Tokio task associated with a module instance.
///
/// Ownership of the module instance is transferred to the
/// spawned Tokio task.
///
/// After this function returns:
/// - MMF owns the command Senders and the JoinHandle.
/// - ModuleTask owns the module instance and the Receiver channels.
///
/// The spawned Tokio task executes the complete module
/// lifecycle, including initialization, message processing
/// and shutdown.
pub(crate) fn spawn_module_task(module: Box<dyn Module + Send>, normal_receiver_channel: ModuleNormalReceiverChannel, priority_receiver_channel: ModulePriorityReceiverChannel, module_id:ModuleID, father_id:EntityID) -> JoinHandle<()> {
   
    tokio::spawn(async move {

        let task_module_id = module_id.clone();
        // Execute the module task while catching Rust panics.
        //
        // Ok(())           => normal termination.
        // Err(payload)     => module panicked.
        //
        // Native crashes (unsafe/DLL) are not recoverable.
        let result = AssertUnwindSafe(module_task(module, normal_receiver_channel, priority_receiver_channel, module_id, father_id)).catch_unwind().await;

        match result {

            Ok(()) => {
                // Nothing to do.
            }

            Err(payload) => {

                let panic_error =
                    if let Some(message) = payload.downcast_ref::<&str>() {

                        anyhow::anyhow!("{}", message)
                    } 
                    else if let Some(message) = payload.downcast_ref::<String>() {

                        anyhow::anyhow!("{}", message)
                    } 
                    else {

                        anyhow::anyhow!("Unknown panic")
                    };

                // Store the panic in MMFErr.
                MMFErr::add_error_internal(&EntityID::Module(task_module_id.clone()), Some("on_init"), &panic_error).expect(MMFPanic::ErrorBuffer_AddErrorFailed.message());
                // Log the panic in MMFLog
                MMFLog::add_log(Instant::now(), &task_module_id, LogAction::ModulePanic, LogStatus::Error).expect(MMFPanic::LogBuffer_AddLogFailed.message());
    
                let mut mmf_data_guard = MMF_MOD_DATA.write().expect(MMFPanic::MMFModData_WriteLockPoisoned.message());
                let mmf_data = mmf_data_guard.as_mut().expect(MMFPanic::MMFModData_Missing.message());

                // Update the module state.
                mmf_data.module_registry.set_module_state(&task_module_id, ModuleState::Panicked).expect(MMFPanic::Registry_ModuleMissing.message());

                // Remove the module from the Registry.
                mmf_data.module_registry.remove_module(&task_module_id).expect(MMFPanic::Registry_ModuleMissing.message());

                drop(mmf_data_guard);

                // Wake up all waiters interested in module
                // shutdown events.
                MMF_MOD_CLOSE_NOTIFY.notify_waiters();
            }
        }

    })    
}


/// Execute the complete lifecycle of a module.
///
/// This async function is executed by the Tokio task created by
/// spawn_module_task().
///
/// The module instance is created by the registered
/// ModuleConstructor before being transferred to this task.
///
/// Responsibilities:
/// - Own the module instance.
/// - Receive lifecycle commands and business messages.
/// - Execute module callbacks.
/// - Update the Registry.
/// - Report errors and log module activity.
/// - Terminate when the module is closed.
async fn module_task(module: Box<dyn Module + Send>, normal_receiver_channel: ModuleNormalReceiverChannel, priority_receiver_channel: ModulePriorityReceiverChannel, module_id: ModuleID, father_id: EntityID) {

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
                        command.expect(MMFPanic::Module_PriorityChannelClosed.message())
                    }
                    command = normal_receiver_channel.recv() => {
                        command.expect(MMFPanic::Module_NormalChannelClosed.message())
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
            ModuleCommand::Init {lifecycle_data} => {

                let action_start_timer = Instant::now();
                MMFLog::add_log(action_start_timer.clone(), &module_id, LogAction::ModuleInit, LogStatus::Start).expect(MMFPanic::LogBuffer_AddLogFailed.message());


                let result_init = module.on_init(module_id.clone(), father_id.clone(), lifecycle_data).await;

                // Publish successful initialization.
                //
                // The module is now fully initialized and can
                // receive business messages.
                let mut mmf_data_guard = MMF_MOD_DATA.write().expect(MMFPanic::MMFModData_WriteLockPoisoned.message());
                let mmf_data = mmf_data_guard.as_mut().expect(MMFPanic::MMFModData_Missing.message());
                match &result_init {
                    Ok(()) => {

                        mmf_data.module_registry.set_module_state(&module_id, ModuleState::Initialized).expect(MMFPanic::Registry_ModuleMissing.message());
                        MMFLog::add_log(action_start_timer, &module_id, LogAction::ModuleInit, LogStatus::Ok).expect(MMFPanic::LogBuffer_AddLogFailed.message());
                    }
                    Err(error) => {

                        MMFErr::add_error_internal(&EntityID::Module(module_id.clone()), Some("on_init"), error).expect(MMFPanic::ErrorBuffer_AddErrorFailed.message());
                        mmf_data.module_registry.set_module_state(&module_id, ModuleState::InitFailed).expect(MMFPanic::Registry_ModuleMissing.message());
                        MMFLog::add_log(action_start_timer, &module_id, LogAction::ModuleInit, LogStatus::Error).expect(MMFPanic::LogBuffer_AddLogFailed.message());
                    }

                }
                drop(mmf_data_guard);

                // Wake up all waiters interested in module
                // initialization events.
                MMF_MOD_INIT_NOTIFY.notify_waiters();
            }

            // Execute a business message.
            //
            // Messages are processed through the same FIFO queue
            // as lifecycle commands (like Init or Close).
            //
            // This guarantees that a message accepted before a
            // Close command is always processed before module
            // shutdown begins.
            ModuleCommand::Message {message, priority} => {

                let action_start_timer = Instant::now();
                let log_action = LogAction::ModuleMessage {

                    sender_id: message.sender_id.clone(),
                    message_id: message.message_id.clone(),
                    priority,
                    wait_completion: false,
                };
                MMFLog::add_log(action_start_timer.clone(), &module_id, log_action.clone(), LogStatus::Start).expect(MMFPanic::LogBuffer_AddLogFailed.message());

                let result_message = module.on_message(message).await;
                
                match &result_message {

                    Ok(()) => {

                        MMFLog::add_log(action_start_timer, &module_id, log_action, LogStatus::Ok).expect(MMFPanic::LogBuffer_AddLogFailed.message());
                    }

                    Err(error) => {

                        MMFErr::add_error_internal(&EntityID::Module(module_id.clone()), Some("on_message"), error).expect(MMFPanic::ErrorBuffer_AddErrorFailed.message());
                        MMFLog::add_log(action_start_timer, &module_id, log_action, LogStatus::Error).expect(MMFPanic::LogBuffer_AddLogFailed.message());
                    }
                }
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
            ModuleCommand::MessageAndWaitCompletion {reply_sender, message, priority} => {

                let action_start_timer = Instant::now();
                let log_action = LogAction::ModuleMessage {

                    sender_id: message.sender_id.clone(),
                    message_id: message.message_id.clone(),
                    priority,
                    wait_completion: true,
                };
                MMFLog::add_log(action_start_timer.clone(), &module_id, log_action.clone(), LogStatus::Start).expect(MMFPanic::LogBuffer_AddLogFailed.message());

                let result_message = module.on_message(message).await;

                match &result_message {

                    Ok(()) => {

                        MMFLog::add_log(action_start_timer, &module_id, log_action, LogStatus::Ok).expect(MMFPanic::LogBuffer_AddLogFailed.message());
                    }

                    Err(error) => {

                        MMFErr::add_error_internal(&EntityID::Module(module_id.clone()), Some("on_message"), &error).expect(MMFPanic::ErrorBuffer_AddErrorFailed.message());
                        MMFLog::add_log(action_start_timer, &module_id, log_action, LogStatus::Error).expect(MMFPanic::LogBuffer_AddLogFailed.message());
                    }
                }
                
                let _ = reply_sender.send(result_message);
            }

            // Execute module shutdown.
            //
            // MMF has already changed the module state from
            // Initialized to Closing before sending this command.
            //
            // ModuleTask is responsible for removing the module
            // from the Registry only after on_close() completes.
            ModuleCommand::Close {lifecycle_data} => {

                let action_start_timer = Instant::now();
                MMFLog::add_log(action_start_timer.clone(), &module_id, LogAction::ModuleClose, LogStatus::Start).expect(MMFPanic::LogBuffer_AddLogFailed.message());

                let result_close = module.on_close(lifecycle_data).await;

                // Mark the module as Closed.
                //
                // Closed indicates that the module lifecycle
                // completed successfully.
                let mut mmf_data_guard = MMF_MOD_DATA.write().expect(MMFPanic::MMFModData_WriteLockPoisoned.message());
                let mmf_data = mmf_data_guard.as_mut().expect(MMFPanic::MMFModData_Missing.message());
                match &result_close {
                    Ok(()) => {

                        mmf_data.module_registry.set_module_state(&module_id, ModuleState::Closed).expect(MMFPanic::Registry_ModuleMissing.message());
                        MMFLog::add_log(action_start_timer, &module_id, LogAction::ModuleClose, LogStatus::Ok).expect(MMFPanic::LogBuffer_AddLogFailed.message());
                        // Remove the module from the Registry.
                        //
                        // Once removed, the module is no longer visible
                        // to MMF and cannot receive new commands.
                        mmf_data.module_registry.remove_module(&module_id).expect(MMFPanic::Registry_ModuleMissing.message());
                        let _ = MMFLog::remove_logged_module(&module_id);
                    }
                    Err(error) => {

                        MMFErr::add_error_internal(&EntityID::Module(module_id.clone()), Some("on_close"), error).expect(MMFPanic::ErrorBuffer_AddErrorFailed.message());
                        mmf_data.module_registry.set_module_state(&module_id, ModuleState::CloseFailed).expect(MMFPanic::Registry_ModuleMissing.message());
                        MMFLog::add_log(action_start_timer, &module_id, LogAction::ModuleClose, LogStatus::Error).expect(MMFPanic::LogBuffer_AddLogFailed.message());
                    }
                }

                drop(mmf_data_guard);

                // Wake up all waiters interested in module
                // shutdown events.
                MMF_MOD_CLOSE_NOTIFY.notify_waiters();

                // Terminate the Tokio task associated with
                // this module.
                break;
            }
        }
        
    }
}