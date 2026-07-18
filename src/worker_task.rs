//! Tokio runtime for MWF workers.
//!
//! worker_task owns the Tokio task associated with a worker instance.
//!
//! After a worker is spawned:
//! - MWF owns the Senders and the JoinHandle.
//! - WorkerTask owns the worker instance and the Receivers.
//!
//! Responsibilities:
//! - Own the worker instance.
//! - Receive commands and messages sent by MWF.
//! - Execute the worker callbacks.
//! - Terminate the worker task when requested.
//! - Process all worker commands sequentially.
//!
//! worker commands are first collected into an internal queue.
//!
//! Priority commands are inserted using a LIFO policy.
//! Normal commands are inserted using a FIFO policy.
//!
//! Commands are then executed sequentially from this queue.
//!
//! MWF never directly calls on_init(), on_message() or on_close().
//! These callbacks are executed inside the worker task because
//! the worker instance is owned by Tokio after spawn_worker().

use futures::FutureExt;
use std::collections::VecDeque;
use std::panic::AssertUnwindSafe;
use std::time::Instant;
use tokio::task::JoinHandle;

use crate::MWFErr;
use crate::MWFLog;
use crate::channel::{WorkerCommand, WorkerNormalReceiverChannel, WorkerPriorityReceiverChannel};
use crate::framework_error::MWFPanic;
use crate::message::WorkerMessage;
use crate::mwf::{MWF_WRK_CLOSE_NOTIFY, MWF_WRK_DATA, MWF_WRK_INIT_NOTIFY};
use crate::mwf_log::{LogAction, LogStatus};
use crate::worker::{EntityID, Worker, WorkerError, WorkerID, WorkerState};

/// Internal command queue owned by WorkerTask.
///
/// Commands received from the Tokio channels are first inserted
/// into this queue before being executed.
///
/// The queue is responsible for preserving the execution policy
/// of each command type (FIFO, LIFO, unique filtering, ...).
pub(crate) struct PendingCommands {
    commands: VecDeque<WorkerCommand>,
}

impl PendingCommands {
    /// Create an empty pending command queue.
    pub fn new() -> Self {
        Self { commands: VecDeque::new() }
    }

    /// Insert a priority command.
    ///
    /// Priority commands are stored using a LIFO policy.
    pub fn push_priority(&mut self, command: WorkerCommand) {
        self.commands.push_back(command);
    }

    /// Insert a normal command.
    ///
    /// Normal commands are stored using a FIFO policy.
    pub fn push_normal(&mut self, command: WorkerCommand) {
        if command.is_unique_message() {
            self.push_unique(command);
        } else {
            self.commands.push_front(command);
        }
    }

    /// Insert a unique message.
    ///
    /// If another unique message with the same MessageID, regardless
    /// of sender, is already pending, its content is replaced while
    /// preserving its position in the FIFO queue.
    ///
    /// Otherwise, the message is inserted using the normal FIFO
    /// policy.
    fn push_unique(&mut self, command: WorkerCommand) {
        // Search for an already queued unique message with the
        // same MessageID.
        for queued_command in &mut self.commands {
            if queued_command.is_unique_message() && queued_command.message_id() == command.message_id() {
                *queued_command = command;
                return;
            }
        }

        // No equivalent unique message was found.
        //
        // Insert the message using the normal FIFO policy.
        self.commands.push_front(command);
    }

    /// Return the next command to execute.
    pub fn pop_next(&mut self) -> Option<WorkerCommand> {
        self.commands.pop_back()
    }

    /// Return whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

/// Spawn the Tokio task associated with a worker instance.
///
/// Ownership of the worker instance is transferred to the
/// spawned Tokio task.
///
/// After this function returns:
/// - MWF owns the command Senders and the JoinHandle.
/// - WorkerTask owns the worker instance and the Receiver channels.
///
/// The spawned Tokio task executes the complete worker
/// lifecycle, including initialization, message processing
/// and shutdown.
pub(crate) fn spawn_worker_task(worker: Box<dyn Worker + Send>, normal_receiver_channel: WorkerNormalReceiverChannel, priority_receiver_channel: WorkerPriorityReceiverChannel, worker_id: WorkerID, parent_id: EntityID) -> JoinHandle<()> {
    tokio::spawn(async move {
        let task_worker_id = worker_id.clone();
        // Execute the worker task while catching Rust panics.
        //
        // Ok(())           => normal termination.
        // Err(payload)     => worker panicked.
        //
        // Native crashes (unsafe/DLL) are not recoverable.
        let result = AssertUnwindSafe(worker_task(worker, normal_receiver_channel, priority_receiver_channel, worker_id, parent_id)).catch_unwind().await;

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

                // Store the panic in MWFErr.
                MWFErr::add_error_internal(&EntityID::Worker(task_worker_id.clone()), Some("on_init".into()), &panic_error).expect(MWFPanic::ErrorBuffer_AddErrorFailed.message());
                // Log the panic in MWFLog
                MWFLog::add_log(Instant::now(), &task_worker_id, LogAction::WorkerPanic, LogStatus::Error).expect(MWFPanic::LogBuffer_AddLogFailed.message());

                let mut mwf_data_guard = MWF_WRK_DATA.write().expect(MWFPanic::MWFWrkData_WriteLockPoisoned.message());
                let mwf_data = mwf_data_guard.as_mut().expect(MWFPanic::MWFWrkData_Missing.message());

                // Update the worker state.
                mwf_data.worker_registry.set_worker_state(&task_worker_id, WorkerState::Panicked).expect(MWFPanic::Registry_workerMissing.message());

                // Remove the worker from the Registry.
                mwf_data.worker_registry.remove_worker(&task_worker_id).expect(MWFPanic::Registry_workerMissing.message());

                drop(mwf_data_guard);

                // Wake up all waiters interested in worker
                // shutdown events.
                MWF_WRK_CLOSE_NOTIFY.notify_waiters();
            }
        }
    })
}

/// Execute a business message.
///
/// This helper executes the message callback, reports errors and
/// logs the complete message lifecycle.
async fn execute_message(worker: &mut Box<dyn Worker + Send>, worker_id: &WorkerID, message: WorkerMessage, priority: bool, wait_completion: bool) -> Result<(), WorkerError> {
    let action_start_timer = Instant::now();
    let log_action = LogAction::WorkerMessage { message_instance_id: message.message_instance_id, sender_id: message.sender_id.clone(), message_id: message.message_id.clone(), priority, wait_completion };
    let sender_id = message.sender_id.clone();
    let message_id = message.message_id.clone();

    MWFLog::add_log(action_start_timer, worker_id, log_action.clone(), LogStatus::Start).expect(MWFPanic::LogBuffer_AddLogFailed.message());

    let result = worker.on_message(message).await;

    match &result {
        Ok(()) => {
            MWFLog::add_log(action_start_timer, worker_id, log_action, LogStatus::Ok).expect(MWFPanic::LogBuffer_AddLogFailed.message());
        }

        Err(error) => {
            let error_context = format!("on_message -> MessageID={}, SenderID={}", message_id, sender_id);
            MWFErr::add_error_internal(&EntityID::Worker(worker_id.to_owned()), Some(error_context), error).expect(MWFPanic::ErrorBuffer_AddErrorFailed.message());
            MWFLog::add_log(action_start_timer, worker_id, log_action, LogStatus::Error).expect(MWFPanic::LogBuffer_AddLogFailed.message());
        }
    }
    result
}

/// Execute the complete lifecycle of a worker.
///
/// This async function is executed by the Tokio task created by
/// spawn_worker_task().
///
/// The worker instance is created by the registered
/// workerConstructor before being transferred to this task.
///
/// Responsibilities:
/// - Own the worker instance.
/// - Receive lifecycle commands and business messages.
/// - Execute worker callbacks.
/// - Collect commands from the Tokio channels.
/// - Schedule command execution through PendingCommands.
/// - Update the Registry.
/// - Report errors and log worker activity.
/// - Terminate when the worker is closed.
async fn worker_task(worker: Box<dyn Worker + Send>, normal_receiver_channel: WorkerNormalReceiverChannel, priority_receiver_channel: WorkerPriorityReceiverChannel, worker_id: WorkerID, parent_id: EntityID) {
    // WorkerTask owns the worker instance.
    //
    // All worker callbacks are executed
    // inside this Tokio task.
    let mut worker = worker;
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
                        command.expect(MWFPanic::Worker_PriorityChannelClosed.message())
                    );
                }

                command = normal_receiver_channel.recv() => {
                    pending_commands.push_normal(
                        command.expect(MWFPanic::Worker_NormalChannelClosed.message())
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
        let command = pending_commands.pop_next().expect(MWFPanic::Worker_PendingCommandsEmpty.message());

        match command {
            // Execute worker initialization.
            //
            // MWF has already changed the worker state from
            // Created to Initializing before sending this command.
            //
            // WorkerTask is responsible for marking the worker
            // as Initialized only after on_init() completes.
            WorkerCommand::Init { lifecycle_data } => {
                let action_start_timer = Instant::now();
                MWFLog::add_log(action_start_timer, &worker_id, LogAction::WorkerInit, LogStatus::Start).expect(MWFPanic::LogBuffer_AddLogFailed.message());

                let result_init = worker.on_init(worker_id.clone(), parent_id.clone(), lifecycle_data).await;

                // Publish successful initialization.
                //
                // The worker is now fully initialized and can
                // receive business messages.
                let mut mwf_data_guard = MWF_WRK_DATA.write().expect(MWFPanic::MWFWrkData_WriteLockPoisoned.message());
                let mwf_data = mwf_data_guard.as_mut().expect(MWFPanic::MWFWrkData_Missing.message());
                match &result_init {
                    Ok(()) => {
                        mwf_data.worker_registry.set_worker_state(&worker_id, WorkerState::Initialized).expect(MWFPanic::Registry_workerMissing.message());
                        MWFLog::add_log(action_start_timer, &worker_id, LogAction::WorkerInit, LogStatus::Ok).expect(MWFPanic::LogBuffer_AddLogFailed.message());
                    }
                    Err(error) => {
                        MWFErr::add_error_internal(&EntityID::new_worker(worker_id.clone()), Some("on_init".into()), error).expect(MWFPanic::ErrorBuffer_AddErrorFailed.message());
                        mwf_data.worker_registry.set_worker_state(&worker_id, WorkerState::InitFailed).expect(MWFPanic::Registry_workerMissing.message());
                        MWFLog::add_log(action_start_timer, &worker_id, LogAction::WorkerInit, LogStatus::Error).expect(MWFPanic::LogBuffer_AddLogFailed.message());
                    }
                }
                drop(mwf_data_guard);

                // Wake up all waiters interested in worker
                // initialization events.
                MWF_WRK_INIT_NOTIFY.notify_waiters();
            }

            // Execute a business message.
            //
            // Messages are processed through the same FIFO queue
            // as lifecycle commands (like Init or Close).
            //
            // This guarantees that a message accepted before a
            // Close command is always processed before worker
            // shutdown begins.
            WorkerCommand::Message { message, priority, unique: _ } => {
                let _ = execute_message(&mut worker, &worker_id, message, priority, false).await;
            }

            // Execute a business message.
            //
            // MWF is blocked waiting for confirmation that on_message()
            // has completed.
            //
            // Messages are processed through the same FIFO queue
            // as lifecycle commands (like Init or Close).
            //
            // This guarantees that a message accepted before a
            // Close command is always processed before worker
            // shutdown begins.
            WorkerCommand::MessageAndWaitCompletion { reply_sender, message, priority } => {
                let result_message = execute_message(&mut worker, &worker_id, message, priority, true).await;

                let _ = reply_sender.send(result_message);
            }

            // Execute worker shutdown.
            //
            // MWF has already changed the worker state from
            // Initialized to Closing before sending this command.
            //
            // WorkerTask is responsible for removing the worker
            // from the Registry only after on_close() completes.
            WorkerCommand::Close { lifecycle_data } => {
                let action_start_timer = Instant::now();
                MWFLog::add_log(action_start_timer, &worker_id, LogAction::WorkerClose, LogStatus::Start).expect(MWFPanic::LogBuffer_AddLogFailed.message());

                let result_close = worker.on_close(lifecycle_data).await;

                // Mark the worker as Closed.
                //
                // Closed indicates that the worker lifecycle
                // completed successfully.
                let mut mwf_data_guard = MWF_WRK_DATA.write().expect(MWFPanic::MWFWrkData_WriteLockPoisoned.message());
                let mwf_data = mwf_data_guard.as_mut().expect(MWFPanic::MWFWrkData_Missing.message());
                match &result_close {
                    Ok(()) => {
                        mwf_data.worker_registry.set_worker_state(&worker_id, WorkerState::Closed).expect(MWFPanic::Registry_workerMissing.message());
                        MWFLog::add_log(action_start_timer, &worker_id, LogAction::WorkerClose, LogStatus::Ok).expect(MWFPanic::LogBuffer_AddLogFailed.message());
                        // Remove the worker from the Registry.
                        //
                        // Once removed, the worker is no longer visible
                        // to MWF and cannot receive new commands.
                        mwf_data.worker_registry.remove_worker(&worker_id).expect(MWFPanic::Registry_workerMissing.message());
                        let _ = MWFLog::remove_logged_worker(&worker_id);
                    }
                    Err(error) => {
                        MWFErr::add_error_internal(&EntityID::new_worker(worker_id.clone()), Some("on_close".into()), error).expect(MWFPanic::ErrorBuffer_AddErrorFailed.message());
                        mwf_data.worker_registry.set_worker_state(&worker_id, WorkerState::CloseFailed).expect(MWFPanic::Registry_workerMissing.message());
                        MWFLog::add_log(action_start_timer, &worker_id, LogAction::WorkerClose, LogStatus::Error).expect(MWFPanic::LogBuffer_AddLogFailed.message());
                    }
                }

                drop(mwf_data_guard);

                // Wake up all waiters interested in worker
                // shutdown events.
                MWF_WRK_CLOSE_NOTIFY.notify_waiters();

                // Terminate the Tokio task associated with
                // this worker.
                break;
            }
        }
    }
}
