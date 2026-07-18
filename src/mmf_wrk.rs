//! Public MMFWrk (worker and messaging) API.
//!
//! This API exposes the framework entry points used to
//! manage workers and send messages.

use bytes::Bytes;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use crate::MMFLog;
use crate::channel::{WorkerChannel, WorkerCommand, get_sender_channel};
use crate::framework_error::{MMFError, MMFPanic};
use crate::message::{MessageID, MessageInstanceID, WorkerMessage};
use crate::mmf::{MMF_DATA, MMF_WRK_CLOSE_NOTIFY, MMF_WRK_DATA, MMF_WRK_INIT_NOTIFY, MMF_WRK_MESSAGE_INSTANCE_COUNTER, MMFConfiguration};
use crate::mmf_log::{LogAction, LogStatus};
use crate::worker::{EntityID, Worker, WorkerError, WorkerID, WorkerList, WorkerState, WorkerStatus, WorkerTypeID};
use crate::worker_registry::Registry;
use crate::worker_task::spawn_worker_task;

/// Internal MMFWrk runtime data.
#[derive(Debug)]
pub(crate) struct WorkerData {
    /// Registered workers and hierarchy information.
    pub(crate) worker_registry: Registry,

    /// Capacity of normal worker event queues.
    pub(crate) worker_normal_channel_capacity: usize,

    /// Capacity of priority worker event queues.
    pub(crate) worker_priority_channel_capacity: usize,
}

impl WorkerData {
    /// Create a new MMF runtime instance.
    pub(crate) fn new(mmf_configuration: &MMFConfiguration) -> Self {
        Self { worker_registry: Registry::new(), worker_normal_channel_capacity: mmf_configuration.worker_normal_channel_capacity, worker_priority_channel_capacity: mmf_configuration.worker_priority_channel_capacity }
    }
}

pub struct MMFWrk;

/// Public MMFWrk entry point.
impl MMFWrk {
    /// Generate a unique MessageInstanceID.
    fn generate_message_instance_id() -> Result<MessageInstanceID, MMFError> {
        let current_id = MMF_WRK_MESSAGE_INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed);
        if current_id == u64::MAX {
            return Err(MMFError::Message_InstanceIDExhausted);
        }

        Ok(current_id)
    }

    /// Create and register a new worker instance from its WorkerTypeID.
    ///
    /// The worker type must have been previously
    /// registered using MMF::register_worker_type().
    ///
    /// MMF retrieves the registered worker constructor,
    /// creates a new worker instance, then spawns it.
    pub fn spawn_worker(worker_type_id: WorkerTypeID, parent_id: &EntityID) -> Result<WorkerID, MMFError> {
        // Ensure the framework has been initialized.
        let mmf_data_guard = MMF_DATA.read().expect(MMFPanic::MMFData_ReadLockPoisoned.message());
        let mmf_data = mmf_data_guard.as_ref().ok_or(MMFError::Framework_NotInitialized)?;

        // Ensure the worker type has been registered.
        if !mmf_data.worker_type_registry.contains_worker_type(worker_type_id) {
            return Err(MMFError::WorkerType_NotRegistered);
        }

        // Retrieve the constructor associated with the WorkerTypeID.
        let worker_constructor = mmf_data.worker_type_registry.get_worker_constructor(worker_type_id)?;

        // Create a new worker instance.
        let worker = worker_constructor();

        drop(mmf_data_guard);

        MMFWrk::spawn_worker_instance(worker_type_id, worker, parent_id)
    }

    /// Spawn and register a worker instance.
    ///
    /// The worker instance must have been created from a
    /// previously registered WorkerTypeID.
    ///
    /// Steps:
    /// 1. Generate a unique WorkerID.
    /// 2. Create communication channels used by MMF.
    /// 3. Spawn the Tokio task that will own the worker instance.
    /// 4. Register the worker resources in the Registry.
    ///
    /// After this function returns:
    /// - MMF owns the WorkerID, channels and JoinHandle.
    /// - The Tokio task owns the worker instance itself.
    pub(crate) fn spawn_worker_instance(worker_type_id: WorkerTypeID, worker: Box<dyn Worker + Send>, parent_id: &EntityID) -> Result<WorkerID, MMFError> {
        let mut mmf_data_guard = MMF_WRK_DATA.write().expect(MMFPanic::MMFWrkData_WriteLockPoisoned.message());
        let mmf_data = mmf_data_guard.as_mut().ok_or(MMFError::Framework_NotInitialized)?;

        // Child workers can only be attached to an
        // initialized parent worker.
        //
        // This prevents creating new hierarchy branches
        // while the parent is starting up or shutting down.
        match parent_id {
            EntityID::Worker(parent_id_ref) => {
                let worker_state = mmf_data.worker_registry.get_worker_state(parent_id_ref)?;

                if worker_state != WorkerState::Initialized {
                    return Err(MMFError::Worker_NotInitialized);
                }
            }

            EntityID::External(_) => {}
        }

        let worker_id = mmf_data.worker_registry.generate_worker_id(worker_type_id);

        let action_start_timer = Instant::now();

        // Channel used to send worker commands (like Init and Close) and messages to the worker.
        let (normal_sender_channel, normal_receiver_channel) = tokio::sync::mpsc::channel(mmf_data.worker_normal_channel_capacity);
        // Channel used to send priority commands and messages to the worker.
        let (priority_sender_channel, priority_receiver_channel) = tokio::sync::mpsc::channel(mmf_data.worker_priority_channel_capacity);

        // Spawn the Tokio task that will own the worker instance.
        //
        // Once spawned, MMF no longer owns the worker.
        // MMF communicates with the worker through channels and
        // tracks its execution through the JoinHandle.
        let join_handle = spawn_worker_task(worker, normal_receiver_channel, priority_receiver_channel, worker_id.clone(), parent_id.clone());

        // Store the worker resources in the Registry.
        //
        // The Registry stores:
        // - worker hierarchy information.
        // - worker state.
        // - Communication channels.
        // - JoinHandle associated with the Tokio task.
        match mmf_data.worker_registry.add_worker(&worker_id, worker_type_id, parent_id, normal_sender_channel, priority_sender_channel, join_handle) {
            Ok(()) => {}

            Err((error, join_handle)) => {
                // Abort the spawned Tokio task to avoid leaving
                // an orphan worker running.
                join_handle.abort();

                MMFLog::add_log(action_start_timer, &worker_id, LogAction::WorkerSpawn, LogStatus::Error)?;

                return Err(error);
            }
        }

        MMFLog::add_log(action_start_timer, &worker_id, LogAction::WorkerSpawn, LogStatus::Ok)?;

        drop(mmf_data_guard);

        Ok(worker_id)
    }

    /// Execute blocking work without blocking the MMF runtime.
    ///
    /// Typical uses include native DLL calls and CPU-intensive computations.
    ///
    /// The blocking work is executed on Tokio's blocking thread pool,
    /// allowing the Tokio runtime to continue executing other MMF workers.
    pub async fn spawn_blocking<F, R>(f: F) -> Result<R, MMFError>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        tokio::task::spawn_blocking(f).await.map_err(|e| MMFError::BlockingTask_Failed(anyhow::anyhow!(e)))
    }

    /// Suspend the current worker for the specified duration.
    ///
    /// While this worker is suspended, the Tokio runtime can continue
    /// executing other MMF workers.
    pub async fn sleep(duration: Duration) {
        tokio::time::sleep(duration).await;
    }

    /// Get the current status of a worker.
    pub fn get_worker_status(worker_id: &WorkerID) -> Result<WorkerStatus, MMFError> {
        let mmf_data_guard = MMF_WRK_DATA.read().expect(MMFPanic::MMFWrkData_ReadLockPoisoned.message());
        let mmf_data = mmf_data_guard.as_ref().ok_or(MMFError::Framework_NotInitialized)?;
        let registry_entry = mmf_data.worker_registry.get_registry_entry(worker_id)?;
        let normal_channel_capacity = mmf_data.worker_normal_channel_capacity;
        let priority_channel_capacity = mmf_data.worker_priority_channel_capacity;
        let normal_channel_size = normal_channel_capacity - registry_entry.normal_sender_channel.capacity();
        let priority_channel_size = priority_channel_capacity - registry_entry.priority_sender_channel.capacity();

        let worker_status = WorkerStatus { worker_state: registry_entry.worker_state, normal_channel_size, normal_channel_capacity, priority_channel_size, priority_channel_capacity };

        Ok(worker_status)
    }

    /// Retrieve information about all workers currently
    /// registered in MMF.
    ///
    /// The returned list contains a snapshot of the Registry
    /// state at the time of the call.
    ///
    /// This function is primarily intended for:
    /// - Debug tools.
    /// - Monitoring interfaces.
    /// - Runtime inspection.
    ///
    /// Returns Framework_NotInitialized if MMF has not
    /// been initialized through MMF::init().
    pub fn get_worker_info_list() -> Result<WorkerList, MMFError> {
        let mmf_data_guard = MMF_WRK_DATA.read().expect(MMFPanic::MMFWrkData_ReadLockPoisoned.message());
        let mmf_data = mmf_data_guard.as_ref().ok_or(MMFError::Framework_NotInitialized)?;

        Ok(WorkerList { worker_info_list: mmf_data.worker_registry.get_worker_info_list() })
    }

    /// Returns the fully qualified Rust type path associated with a worker.
    ///
    /// This function is intended for diagnostics and debugging. It can help
    /// locate the worker implementation in the application's source code.
    ///
    /// The returned path is an implementation detail and should not be used
    /// for application logic.
    ///
    /// Returns:
    /// - Framework_NotInitialized if MMF has not been initialized.
    /// - worker_NotFound if the worker does not exist.
    pub fn get_worker_type_rust_path(worker_id: &WorkerID) -> Result<&'static str, MMFError> {
        // Get the WorkerTypeID associated with the worker.
        let mmf_mod_data_guard = MMF_WRK_DATA.read().expect(MMFPanic::MMFWrkData_ReadLockPoisoned.message());
        let mmf_mod_data = mmf_mod_data_guard.as_ref().ok_or(MMFError::Framework_NotInitialized)?;
        let worker_type_id = mmf_mod_data.worker_registry.get_worker_type_id(worker_id)?;
        drop(mmf_mod_data_guard);

        // Get the Rust type name associated with the WorkerTypeID.
        let mmf_data_guard = MMF_DATA.read().expect(MMFPanic::MMFData_ReadLockPoisoned.message());
        let mmf_data = mmf_data_guard.as_ref().ok_or(MMFError::Framework_NotInitialized)?;
        let worker_rust_type_name = mmf_data.worker_type_registry.get_worker_rust_type_name(worker_type_id)?;

        Ok(worker_rust_type_name)
    }

    /// Request worker initialization.
    ///
    /// The worker state is immediately changed to Initializing.
    /// This function only queues the Init command and does not wait
    /// for on_init() completion.
    pub async fn init_worker(worker_id: &WorkerID, lifecycle_data: Bytes) -> Result<(), MMFError> {
        // Change worker state to Initializing
        // and read MMF data to retrieve the worker channel
        // associated with the target worker.
        //
        // Only workers in the Created state can be initialized.
        let normal_sender_channel = {
            let mut mmf_data_guard = MMF_WRK_DATA.write().expect(MMFPanic::MMFWrkData_WriteLockPoisoned.message());
            let mmf_data = mmf_data_guard.as_mut().ok_or(MMFError::Framework_NotInitialized)?;
            let worker_state = mmf_data.worker_registry.get_worker_state(worker_id)?;
            if worker_state != WorkerState::Created {
                return Err(MMFError::Worker_AlreadyInitialized);
            }
            mmf_data.worker_registry.set_worker_state(worker_id, WorkerState::Initializing)?;

            mmf_data.worker_registry.get_worker_normal_channel(worker_id)?
        };

        // Send the Init command to WorkerTask.
        normal_sender_channel.send(WorkerCommand::Init { lifecycle_data }).await.map_err(|_| MMFError::Worker_ChannelClosed)?;

        Ok(())
    }

    /// Request worker shutdown.
    ///
    /// The worker state is immediately changed to Closing.
    /// This function only queues the Close command and does not wait
    /// for on_close() completion.
    pub async fn close_worker(worker_id: &WorkerID, lifecycle_data: Bytes) -> Result<(), MMFError> {
        // Change worker state to Closing
        // and read MMF data to retrieve the worker channel
        // associated with the target worker.
        //
        // Only workers in the Initialized state can be closed.
        let normal_sender_channel = {
            let mut mmf_data_guard = MMF_WRK_DATA.write().expect(MMFPanic::MMFWrkData_WriteLockPoisoned.message());
            let mmf_data = mmf_data_guard.as_mut().ok_or(MMFError::Framework_NotInitialized)?;
            let worker_state = mmf_data.worker_registry.get_worker_state(worker_id)?;
            if worker_state != WorkerState::Initialized {
                return Err(MMFError::Worker_NotInitialized);
            }
            mmf_data.worker_registry.set_worker_state(worker_id, WorkerState::Closing)?;
            mmf_data.worker_registry.get_worker_normal_channel(worker_id)?
        };

        // Send the Close command to WorkerTask.
        normal_sender_channel.send(WorkerCommand::Close { lifecycle_data }).await.map_err(|_| MMFError::Worker_ChannelClosed)?;

        Ok(())
    }

    /// Get the ParentID of a worker.
    pub fn get_parent_id(worker_id: &WorkerID) -> Result<EntityID, MMFError> {
        let mmf_data_guard = MMF_WRK_DATA.read().expect(MMFPanic::MMFWrkData_ReadLockPoisoned.message());
        let mmf_data = mmf_data_guard.as_ref().ok_or(MMFError::Framework_NotInitialized)?;
        let parent_id = mmf_data.worker_registry.get_parent_id(worker_id)?;
        drop(mmf_data_guard);

        Ok(parent_id)
    }

    /// Get the child WorkerIDs of an entity.
    pub fn get_children_ids(entity_id: &EntityID) -> Result<Vec<WorkerID>, MMFError> {
        let mmf_data_guard = MMF_WRK_DATA.read().expect(MMFPanic::MMFWrkData_ReadLockPoisoned.message());
        let mmf_data = mmf_data_guard.as_ref().ok_or(MMFError::Framework_NotInitialized)?;

        mmf_data.worker_registry.get_children_ids(entity_id)
    }

    /// Get the number of child workers of an entity.
    pub fn get_children_count(entity_id: &EntityID) -> Result<usize, MMFError> {
        let mmf_data_guard = MMF_WRK_DATA.read().expect(MMFPanic::MMFWrkData_ReadLockPoisoned.message());
        let mmf_data = mmf_data_guard.as_ref().ok_or(MMFError::Framework_NotInitialized)?;
        let children_count = mmf_data.worker_registry.get_children_count(entity_id)?;
        drop(mmf_data_guard);
        Ok(children_count)
    }

    /// Wait until all workers in the list reach the Initialized state.
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    pub async fn wait_worker_list_init(worker_ids: &Vec<WorkerID>, timeout_ms: Option<u64>) -> Result<(), MMFError> {
        let wait_future = async {
            loop {
                // Register interest in the next initialization event
                // before checking worker states to avoid lost notifications.
                let init_notify = MMF_WRK_INIT_NOTIFY.notified();

                // Read Registry state under a short-lived lock.
                //
                // The lock must be released before awaiting the
                // next notification.
                let (all_init_finished, one_init_failed) = {
                    let mmf_data_guard = MMF_WRK_DATA.read().expect(MMFPanic::MMFWrkData_ReadLockPoisoned.message());
                    let mmf_data = mmf_data_guard.as_ref().ok_or(MMFError::Framework_NotInitialized)?;

                    let mut all_init_finished = true;
                    let mut one_init_failed = false;

                    // Initialization is complete only when all workers
                    // have reached the Initialized or InitFailed state.
                    for worker_id in worker_ids {
                        match mmf_data.worker_registry.get_worker_state(worker_id)? {
                            WorkerState::Initialized => {}

                            WorkerState::InitFailed => {
                                one_init_failed = true;
                            }

                            _ => {
                                all_init_finished = false;
                            }
                        }
                    }

                    (all_init_finished, one_init_failed)
                };

                if all_init_finished {
                    if one_init_failed {
                        return Err(MMFError::Worker_InitializationFailed);
                    }

                    return Ok(());
                }

                // Wait for the next worker initialization event.
                init_notify.await;
            }
        };

        match timeout_ms {
            Some(timeout_ms) => tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), wait_future).await.map_err(|_| MMFError::Operation_Timeout)?,
            None => wait_future.await,
        }
    }

    /// Wait until a worker reaches the Initialized state.
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    pub async fn wait_worker_init(worker_id: &WorkerID, timeout_ms: Option<u64>) -> Result<(), MMFError> {
        MMFWrk::wait_worker_list_init(&vec![worker_id.to_owned()], timeout_ms).await
    }

    /// Wait until all direct children of an entity reach the
    /// Initialized state.
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    pub async fn wait_children_init(entity_id: &EntityID, timeout_ms: Option<u64>) -> Result<(), MMFError> {
        // Resolve the current child list and delegate the wait
        // operation to wait_worker_list_init().
        MMFWrk::wait_worker_list_init(&MMFWrk::get_children_ids(entity_id)?, timeout_ms).await
    }

    /// Wait until all workers in the list have fully closed.
    ///
    /// A worker is considered closed once it has been removed
    /// from the Registry by WorkerTask.
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    pub async fn wait_worker_list_close(worker_ids: &Vec<WorkerID>, timeout_ms: Option<u64>) -> Result<(), MMFError> {
        let wait_future = async {
            loop {
                // Register interest in the next closing event
                // before checking worker states to avoid lost notifications.
                let close_notify = MMF_WRK_CLOSE_NOTIFY.notified();

                // Read Registry state under a short-lived lock.
                //
                // The lock must be released before awaiting the
                // next notification.
                let (all_close_finished, one_close_failed) = {
                    let mmf_data_guard = MMF_WRK_DATA.read().expect(MMFPanic::MMFWrkData_ReadLockPoisoned.message());
                    let mmf_data = mmf_data_guard.as_ref().ok_or(MMFError::Framework_NotInitialized)?;

                    let mut all_close_finished = true;
                    let mut one_close_failed = false;

                    // Closing is complete only when all workers
                    // have either been removed from the Registry
                    // or reached the CloseFailed or the Panicked state.
                    for worker_id in worker_ids {
                        match mmf_data.worker_registry.get_worker_state(worker_id) {
                            // A worker not found has been properly closed.
                            Err(MMFError::Worker_NotFound) => {}

                            Ok(WorkerState::CloseFailed) => {
                                one_close_failed = true;
                            }

                            _ => {
                                all_close_finished = false;
                            }
                        }
                    }

                    (all_close_finished, one_close_failed)
                };

                if all_close_finished {
                    if one_close_failed {
                        return Err(MMFError::Worker_CloseFailed);
                    }

                    return Ok(());
                }

                close_notify.await;
            }
        };

        match timeout_ms {
            Some(timeout_ms) => tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), wait_future).await.map_err(|_| MMFError::Operation_Timeout)?,
            None => wait_future.await,
        }
    }

    /// Wait until a worker has fully closed.
    ///
    /// A worker is considered closed once it has been removed
    /// from the Registry by WorkerTask.
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    pub async fn wait_worker_close(worker_id: &WorkerID, timeout_ms: Option<u64>) -> Result<(), MMFError> {
        MMFWrk::wait_worker_list_close(&vec![worker_id.to_owned()], timeout_ms).await
    }

    /// Wait until all direct children of an entity have fully closed.
    ///
    /// A child is considered closed once it has been removed from
    /// the Registry by WorkerTask.
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    pub async fn wait_children_close(entity_id: &EntityID, timeout_ms: Option<u64>) -> Result<(), MMFError> {
        // Resolve the current child list and delegate the wait
        // operation to wait_worker_list_close().
        MMFWrk::wait_worker_list_close(&MMFWrk::get_children_ids(entity_id)?, timeout_ms).await
    }

    /// Forcefully terminate a list of workers.
    ///
    /// This operation aborts the Tokio tasks associated with the
    /// workers and removes them from the Registry without executing
    /// any additional worker code.
    ///
    /// This function should only be used as a recovery mechanism
    /// when normal shutdown has failed.
    pub fn abort_worker_list(worker_ids: &Vec<WorkerID>) -> Result<(), MMFError> {
        let mut mmf_data_guard = MMF_WRK_DATA.write().expect(MMFPanic::MMFWrkData_WriteLockPoisoned.message());
        let mmf_data = mmf_data_guard.as_mut().ok_or(MMFError::Framework_NotInitialized)?;

        for worker_id in worker_ids {
            let action_start_timer = Instant::now();

            let join_handle = match mmf_data.worker_registry.take_join_handle(worker_id) {
                Ok(join_handle) => join_handle,

                Err(error) => {
                    MMFLog::add_log(action_start_timer, worker_id, LogAction::WorkerAbort, LogStatus::Error)?;
                    return Err(error);
                }
            };

            // Abort the Tokio task immediately.
            join_handle.abort();

            // Remove the worker from the Registry.
            if let Err(error) = mmf_data.worker_registry.remove_worker(worker_id) {
                MMFLog::add_log(action_start_timer, worker_id, LogAction::WorkerAbort, LogStatus::Error)?;
                return Err(error);
            }

            MMFLog::add_log(action_start_timer, worker_id, LogAction::WorkerAbort, LogStatus::Ok)?;
            let _ = MMFLog::remove_logged_worker(worker_id);
        }

        drop(mmf_data_guard);

        // Wake up all waiters interested in worker
        // shutdown events.
        MMF_WRK_CLOSE_NOTIFY.notify_waiters();

        Ok(())
    }

    /// Forcefully terminate a worker.
    ///
    /// This operation aborts the Tokio tasks associated with the
    /// worker and removes them from the Registry without executing
    /// any additional worker code.
    ///
    /// This function should only be used as a recovery mechanism
    /// when normal shutdown has failed.
    pub fn abort_worker(worker_id: &WorkerID) -> Result<(), MMFError> {
        MMFWrk::abort_worker_list(&vec![worker_id.to_owned()])
    }

    /// Forcefully terminate all direct children workers of an entity.
    ///
    /// This operation aborts the Tokio tasks associated with the
    /// workers and removes them from the Registry without executing
    /// any additional worker code.
    ///
    /// This function should only be used as a recovery mechanism
    /// when normal shutdown has failed.
    pub fn abort_children(entity_id: &EntityID) -> Result<(), MMFError> {
        MMFWrk::abort_worker_list(&MMFWrk::get_children_ids(entity_id)?)
    }

    /// Send a message to a worker.
    ///
    /// `message_id` may be any type implementing `Into<MessageID>`.
    pub async fn send_message<T>(sender_id: &EntityID, receiver_id: &WorkerID, message_id: T, message_data: Bytes) -> Result<(), MMFError>
    where
        T: Into<MessageID>,
    {
        let message_id = message_id.into();
        let normal_sender_channel = get_sender_channel(sender_id, receiver_id, WorkerChannel::Normal)?;

        // Generate the unique id and build the message
        let message_instance_id = MMFWrk::generate_message_instance_id()?;
        let framework_message = WorkerMessage::new(sender_id.clone(), receiver_id.clone(), message_id.clone(), message_instance_id, message_data);

        // Send the message to the target worker.
        normal_sender_channel.send(WorkerCommand::Message { message: framework_message, priority: false, unique: false }).await.map_err(|_| MMFError::Worker_ChannelClosed)?;

        Ok(())
    }

    /// Send a message and wait until the receiver has finished
    /// processing it.
    ///
    /// `message_id` may be any type implementing `Into<MessageID>`.
    ///
    /// Returns:
    /// - Ok(()) if on_message() completed successfully.
    /// - workerError if on_message() returned an error.
    /// - Operation_Timeout if the timeout expires.
    pub async fn send_message_and_wait<T>(sender_id: &EntityID, receiver_id: &WorkerID, message_id: T, message_data: Bytes, timeout_ms: Option<u64>) -> Result<(), MMFError>
    where
        T: Into<MessageID>,
    {
        let message_id = message_id.into();
        let normal_sender_channel = get_sender_channel(sender_id, receiver_id, WorkerChannel::Normal)?;

        // Generate the unique id and build the message
        let message_instance_id = MMFWrk::generate_message_instance_id()?;
        let framework_message = WorkerMessage::new(sender_id.clone(), receiver_id.clone(), message_id.clone(), message_instance_id, message_data);

        // Create a temporary reply channel used to signal
        // completion of on_message().
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<Result<(), WorkerError>>();

        // Send the message to the receiver worker.
        normal_sender_channel.send(WorkerCommand::MessageAndWaitCompletion { reply_sender, message: framework_message, priority: false }).await.map_err(|_| MMFError::Worker_ChannelClosed)?;

        // Wait for completion or timeout.
        //
        // The reply channel transports the Result returned
        // by the receiver on_message() callback.
        let worker_result: Result<(), WorkerError> = match timeout_ms {
            Some(timeout_ms) => {
                // Wait until the receiver worker finishes
                // processing the message.
                //
                // timeout() returns:
                // - Ok(channel_result) if the receiver replied.
                // - Err(Elapsed) if the timeout expires.
                let timeout_result = tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), reply_receiver).await;

                // Convert timeout errors into MMF errors.
                let channel_result = match timeout_result {
                    Ok(channel_result) => channel_result,
                    Err(_) => {
                        return Err(MMFError::Operation_Timeout);
                    }
                };

                // The receiver worker task has disappeared
                // before sending its completion status.
                //
                // channel_result contains:
                // - Ok(worker_result)
                // - Err(RecvError)
                match channel_result {
                    Ok(worker_result) => worker_result,
                    Err(_) => {
                        return Err(MMFError::Worker_ChannelClosed);
                    }
                }
            }

            None => {
                // Wait indefinitely until the receiver worker
                // finishes processing the message.
                let channel_result = reply_receiver.await;

                // The receiver worker task has disappeared
                // before sending its completion status.
                match channel_result {
                    Ok(worker_result) => worker_result,
                    Err(_) => {
                        return Err(MMFError::Worker_ChannelClosed);
                    }
                }
            }
        };

        // The receiver worker returned an error while
        // processing the message.
        match worker_result {
            Ok(()) => {}
            Err(worker_error) => {
                return Err(MMFError::Worker_MessageError(worker_error));
            }
        }

        Ok(())
    }

    /// Send a priority message to a worker.
    ///
    /// Priority messages are always processed before pending
    /// normal messages.
    ///
    /// `message_id` may be any type implementing `Into<MessageID>`.
    pub async fn send_priority_message<T>(sender_id: &EntityID, receiver_id: &WorkerID, message_id: T, message_data: Bytes) -> Result<(), MMFError>
    where
        T: Into<MessageID>,
    {
        let message_id = message_id.into();
        let priority_sender_channel = get_sender_channel(sender_id, receiver_id, WorkerChannel::Priority)?;

        // Generate the unique id and build the message
        let message_instance_id = MMFWrk::generate_message_instance_id()?;
        let framework_message = WorkerMessage::new(sender_id.clone(), receiver_id.clone(), message_id.clone(), message_instance_id, message_data);

        // Send the priority message to the target worker.
        priority_sender_channel.send(WorkerCommand::Message { message: framework_message, priority: true, unique: false }).await.map_err(|_| MMFError::Worker_ChannelClosed)?;

        Ok(())
    }

    /// Send a priority message and wait until the receiver has finished
    /// processing it.
    ///
    /// Priority messages are always processed before pending
    /// normal messages.
    ///
    /// `message_id` may be any type implementing `Into<MessageID>`.
    ///
    /// Returns:
    /// - Ok(()) if on_message() completed successfully.
    /// - workerError if on_message() returned an error.
    /// - Operation_Timeout if the timeout expires.
    pub async fn send_priority_message_and_wait<T>(sender_id: &EntityID, receiver_id: &WorkerID, message_id: T, message_data: Bytes, timeout_ms: Option<u64>) -> Result<(), MMFError>
    where
        T: Into<MessageID>,
    {
        let message_id = message_id.into();
        let priority_sender_channel = get_sender_channel(sender_id, receiver_id, WorkerChannel::Priority)?;

        // Generate the unique id and build the message
        let message_instance_id = MMFWrk::generate_message_instance_id()?;
        let framework_message = WorkerMessage::new(sender_id.clone(), receiver_id.clone(), message_id.clone(), message_instance_id, message_data);

        // Create a temporary reply channel used to signal
        // completion of on_message().
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel();

        // Send the message to the receiver worker.
        priority_sender_channel.send(WorkerCommand::MessageAndWaitCompletion { reply_sender, message: framework_message, priority: true }).await.map_err(|_| MMFError::Worker_ChannelClosed)?;

        // Wait for completion or timeout.
        //
        // The reply channel transports the Result returned
        // by the receiver on_message() callback.
        let worker_result: Result<(), WorkerError> = match timeout_ms {
            Some(timeout_ms) => {
                // Wait until the receiver worker finishes
                // processing the message.
                //
                // timeout() returns:
                // - Ok(channel_result) if the receiver replied.
                // - Err(Elapsed) if the timeout expires.
                let timeout_result = tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), reply_receiver).await;

                // Convert timeout errors into MMF errors.
                let channel_result = match timeout_result {
                    Ok(channel_result) => channel_result,
                    Err(_) => {
                        return Err(MMFError::Operation_Timeout);
                    }
                };

                // The receiver worker task has disappeared
                // before sending its completion status.
                //
                // channel_result contains:
                // - Ok(worker_result)
                // - Err(RecvError)
                match channel_result {
                    Ok(worker_result) => worker_result,
                    Err(_) => {
                        return Err(MMFError::Worker_ChannelClosed);
                    }
                }
            }

            None => {
                // Wait indefinitely until the receiver worker
                // finishes processing the message.
                let channel_result = reply_receiver.await;

                // The receiver worker task has disappeared
                // before sending its completion status.
                match channel_result {
                    Ok(worker_result) => worker_result,
                    Err(_) => {
                        return Err(MMFError::Worker_ChannelClosed);
                    }
                }
            }
        };

        // The receiver worker returned an error while
        // processing the message.
        match worker_result {
            Ok(()) => {}
            Err(worker_error) => {
                return Err(MMFError::Worker_MessageError(worker_error));
            }
        }

        Ok(())
    }

    /// Send a unique message to a worker.
    ///
    /// At most one pending unique message with the same MessageID
    /// can exist in the receiver queue.
    ///
    /// Sending a new unique message replaces the pending one while
    /// preserving its FIFO position.
    ///
    /// `message_id` may be any type implementing `Into<MessageID>`.
    pub async fn send_unique_message<T>(sender_id: &EntityID, receiver_id: &WorkerID, message_id: T, message_data: Bytes) -> Result<(), MMFError>
    where
        T: Into<MessageID>,
    {
        let message_id = message_id.into();
        let normal_sender_channel = get_sender_channel(sender_id, receiver_id, WorkerChannel::Normal)?;

        // Generate the unique id and build the message
        let message_instance_id = MMFWrk::generate_message_instance_id()?;
        let framework_message = WorkerMessage::new(sender_id.clone(), receiver_id.clone(), message_id.clone(), message_instance_id, message_data);

        // Send the message to the target worker.
        normal_sender_channel.send(WorkerCommand::Message { message: framework_message, priority: false, unique: true }).await.map_err(|_| MMFError::Worker_ChannelClosed)?;

        Ok(())
    }
}
