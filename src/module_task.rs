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
//! Module commands are first collected into an internal queue.
//!
//! Priority commands are inserted using a LIFO policy.
//! Normal commands are inserted using a FIFO policy.
//!
//! Commands are then executed sequentially from this queue.
//!
//! MMF never directly calls on_init(), on_message() or on_close().
//! These callbacks are executed inside the module task because
//! the module instance is owned by Tokio after spawn_module().

use futures::FutureExt;
use std::panic::AssertUnwindSafe;
use std::time::Instant;
use tokio::task::JoinHandle;

use crate::MMFErr;
use crate::MMFLog;
use crate::channel::{ModuleCommand, ModuleNormalReceiverChannel, ModulePriorityReceiverChannel};
use crate::framework_error::MMFPanic;
use crate::mmf::{MMF_MOD_CLOSE_NOTIFY, MMF_MOD_DATA, MMF_MOD_INIT_NOTIFY};
use crate::mmf_log::{LogAction, LogStatus};
use crate::module::{EntityID, Module, ModuleID, ModuleState};

/// Internal command queue owned by ModuleTask.
///
/// Commands received from the Tokio channels are first inserted
/// into this queue before being executed.
///
/// The queue is responsible for preserving the execution policy
/// of each command type (FIFO, LIFO, unique filtering, ...).
pub(crate) struct PendingCommands {
    commands: Vec<ModuleCommand>,
}

impl PendingCommands {
    /// Create an empty pending command queue.
    pub fn new() -> Self {
        Self { commands: Vec::new() }
    }

    /// Insert a priority command.
    ///
    /// Priority commands are stored using a LIFO policy.
    pub fn push_priority(&mut self, command: ModuleCommand) {
        self.commands.push(command);
    }

    /// Insert a normal command.
    ///
    /// Normal commands are stored using a FIFO policy.
    pub fn push_normal(&mut self, command: ModuleCommand) {
        self.commands.insert(0, command);
    }

    /// Return the next command to execute.
    pub fn pop_next(&mut self) -> Option<ModuleCommand> {
        self.commands.pop()
    }

    /// Return whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

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
pub(crate) fn spawn_module_task(module: Box<dyn Module + Send>, normal_receiver_channel: ModuleNormalReceiverChannel, priority_receiver_channel: ModulePriorityReceiverChannel, module_id: ModuleID, father_id: EntityID) -> JoinHandle<()> {
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
                let panic_error = if let Some(message) = payload.downcast_ref::<&str>() {
                    anyhow::anyhow!("{}", message)
                } else if let Some(message) = payload.downcast_ref::<String>() {
                    anyhow::anyhow!("{}", message)
                } else {
                    anyhow::anyhow!("Unknown panic")
                };

                // Store the panic in MMFErr.
                MMFErr::add_error_internal(&EntityID::Module(task_module_id.clone()), Some("on_init".to_string()), &panic_error).expect(MMFPanic::ErrorBuffer_AddErrorFailed.message());
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
/// - Collect commands from the Tokio channels.
/// - Schedule command execution through PendingCommands.
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

    // Local command queue.
    //
    // Commands received from the Tokio channels are first
    // collected into this queue before being executed.
    //
    // The queue preserves the scheduling policy of each
    // command type (FIFO, LIFO, ...).
    let mut pending_commands = PendingCommands::new();

    loop {
        // Wait until at least one command is available.
        //
        // Once awakened, drain both Tokio channels so every
        // currently pending command is collected into the
        // internal queue before execution.
        if pending_commands.is_empty() {
            tokio::select! {

                command = priority_receiver_channel.recv() => {
                    pending_commands.push_priority(
                        command.expect(MMFPanic::Module_PriorityChannelClosed.message())
                    );
                }

                command = normal_receiver_channel.recv() => {
                    pending_commands.push_normal(
                        command.expect(MMFPanic::Module_NormalChannelClosed.message())
                    );
                }
            }

            // Collect all remaining priority commands.
            while let Ok(command) = priority_receiver_channel.try_recv() {
                pending_commands.push_priority(command);
            }

            // Collect all remaining normal commands.
            while let Ok(command) = normal_receiver_channel.try_recv() {
                pending_commands.push_normal(command);
            }
        }

        // Execute the next scheduled command.
        let command = pending_commands.pop_next().expect(MMFPanic::Module_PendingCommandsEmpty.message());

        match command {
            // Execute module initialization.
            //
            // MMF has already changed the module state from
            // Created to Initializing before sending this command.
            //
            // ModuleTask is responsible for marking the module
            // as Initialized only after on_init() completes.
            ModuleCommand::Init { lifecycle_data } => {
                let action_start_timer = Instant::now();
                MMFLog::add_log(action_start_timer, &module_id, LogAction::ModuleInit, LogStatus::Start).expect(MMFPanic::LogBuffer_AddLogFailed.message());

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
                        MMFErr::add_error_internal(&EntityID::Module(module_id.clone()), Some("on_init".to_string()), error).expect(MMFPanic::ErrorBuffer_AddErrorFailed.message());
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
            ModuleCommand::Message { message, priority, unique } => {
                let action_start_timer = Instant::now();
                let log_action = LogAction::ModuleMessage { sender_id: message.sender_id.clone(), message_id: message.message_id.clone(), priority, wait_completion: false };
                let sender_id = message.sender_id.clone();
                let message_id = message.message_id.clone();

                MMFLog::add_log(action_start_timer, &module_id, log_action.clone(), LogStatus::Start).expect(MMFPanic::LogBuffer_AddLogFailed.message());

                let result_message = module.on_message(message).await;

                match &result_message {
                    Ok(()) => {
                        MMFLog::add_log(action_start_timer, &module_id, log_action, LogStatus::Ok).expect(MMFPanic::LogBuffer_AddLogFailed.message());
                    }

                    Err(error) => {
                        let error_context = format!("on_message -> MessageID={}, SenderID={}", message_id, sender_id);
                        MMFErr::add_error_internal(&EntityID::Module(module_id.clone()), Some(error_context.to_string()), error).expect(MMFPanic::ErrorBuffer_AddErrorFailed.message());
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
            ModuleCommand::MessageAndWaitCompletion { reply_sender, message, priority, unique } => {
                let action_start_timer = Instant::now();
                let log_action = LogAction::ModuleMessage { sender_id: message.sender_id.clone(), message_id: message.message_id.clone(), priority, wait_completion: true };
                let sender_id = message.sender_id.clone();
                let message_id = message.message_id.clone();

                MMFLog::add_log(action_start_timer, &module_id, log_action.clone(), LogStatus::Start).expect(MMFPanic::LogBuffer_AddLogFailed.message());

                let result_message = module.on_message(message).await;

                match &result_message {
                    Ok(()) => {
                        MMFLog::add_log(action_start_timer, &module_id, log_action, LogStatus::Ok).expect(MMFPanic::LogBuffer_AddLogFailed.message());
                    }

                    Err(error) => {
                        let error_context = format!("on_message -> MessageID={}, SenderID={}", message_id, sender_id);
                        MMFErr::add_error_internal(&EntityID::Module(module_id.clone()), Some(error_context.to_string()), error).expect(MMFPanic::ErrorBuffer_AddErrorFailed.message());
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
            ModuleCommand::Close { lifecycle_data } => {
                let action_start_timer = Instant::now();
                MMFLog::add_log(action_start_timer, &module_id, LogAction::ModuleClose, LogStatus::Start).expect(MMFPanic::LogBuffer_AddLogFailed.message());

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
                        MMFErr::add_error_internal(&EntityID::Module(module_id.clone()), Some("on_close".to_string()), error).expect(MMFPanic::ErrorBuffer_AddErrorFailed.message());
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
