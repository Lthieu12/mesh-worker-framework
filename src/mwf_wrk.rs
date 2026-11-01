//! Public MWFWrk (worker and messaging) API.
//!
//! This API exposes the framework entry points used to
//! manage workers and send messages.

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use crate::MWFLog;
use crate::channel::{WorkerChannel, WorkerCommand, send_worker_command};
use crate::framework_error::{MWFError, MWFPanic};
use crate::message::{MessageData, MessageFlowID, MessageID, MessageInheritanceContext, MessageInstanceID, MessageWaitMode, WorkerMessage};
use crate::message_wait::{WorkerWaitGuard, WorkerWaitState, complete_message_flow, is_message_flow_active_for_worker, register_worker_wait, wait_message_flow, wait_this_message};
use crate::mwf::{MWF, MWF_DATA, MWF_WRK_CLOSE_NOTIFY, MWF_WRK_DATA, MWF_WRK_INIT_NOTIFY, MWF_WRK_MESSAGE_FLOW_COUNTER, MWF_WRK_MESSAGE_INSTANCE_COUNTER, MWFConfiguration, MWFInstanceID};
use crate::mwf_log::{LogAction, LogStatus};
use crate::worker::{EntityID, Worker, WorkerError, WorkerID, WorkerList, WorkerState, WorkerStatus, WorkerTypeID};
use crate::worker_registry::Registry;
use crate::worker_task::{current_message_inheritance_context, current_parent_id, current_worker_context, current_worker_id, scope_worker_context, spawn_worker_task};

/// Internal MWFWrk runtime data.
#[derive(Debug)]
pub(crate) struct WorkerData {
    /// Registered workers and creation relationships.
    pub(crate) worker_registry: Registry,

    /// Capacity of normal worker event queues.
    pub(crate) worker_normal_channel_capacity: usize,

    /// Capacity of priority worker event queues.
    pub(crate) worker_priority_channel_capacity: usize,
}

impl WorkerData {
    /// Create a new MWF runtime instance.
    pub(crate) fn new(mwf_configuration: &MWFConfiguration, mwf_instance_id: MWFInstanceID) -> Self {
        Self { worker_registry: Registry::new(mwf_instance_id), worker_normal_channel_capacity: mwf_configuration.worker_normal_channel_capacity, worker_priority_channel_capacity: mwf_configuration.worker_priority_channel_capacity }
    }
}

pub struct MWFWrk;

/// Public MWFWrk entry point.
impl MWFWrk {
    /// Generate a unique MessageInstanceID.
    fn generate_message_instance_id() -> Result<MessageInstanceID, MWFError> {
        MWF_WRK_MESSAGE_INSTANCE_COUNTER.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current_id| current_id.checked_add(1)).map_err(|_| MWFError::Message_InstanceIDExhausted)
    }

    /// Generate a unique MessageFlowID.
    fn generate_message_flow_id() -> Result<MessageFlowID, MWFError> {
        MWF_WRK_MESSAGE_FLOW_COUNTER.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current_id| current_id.checked_add(1)).map_err(|_| MWFError::MessageFlow_InstanceIDExhausted)
    }

    /// Returns the current message flow context only for messages sent to the current worker.
    ///
    /// The message flow must still be active and owned by that worker.
    /// Messages sent to another worker never inherit the current message flow.
    fn get_message_inheritance_context_for_receiver(receiver_id: &WorkerID) -> MessageInheritanceContext {
        // Avoid registry access when no message flow is active in the current context.
        let inheritance_context = current_message_inheritance_context();
        let Some(message_flow_id) = inheritance_context.message_flow_id else {
            return MessageInheritanceContext::default();
        };

        // Message flow contexts are only propagated to the current worker itself.
        if current_worker_id().as_ref() != Some(receiver_id) {
            return MessageInheritanceContext::default();
        }

        // Ignore a message flow that has already been closed or belongs to another worker.
        if is_message_flow_active_for_worker(message_flow_id, receiver_id) { inheritance_context } else { MessageInheritanceContext::default() }
    }

    /// Register the current worker's active message wait, when applicable.
    ///
    /// External application tasks have no MWF worker execution context,
    /// so their waits are not added to the worker wait registry.
    fn register_current_worker_wait(receiver_id: &WorkerID, message_id: &MessageID, message_instance_id: MessageInstanceID, wait_mode: MessageWaitMode) -> Result<Option<WorkerWaitGuard>, MWFError> {
        // Identify the worker from the actual execution context.
        let Some(waiting_worker_id) = current_worker_id() else {
            return Ok(None);
        };

        // Register the wait and reject any operation that would create a deadlock.
        let worker_wait_guard = register_worker_wait(&waiting_worker_id, WorkerWaitState { receiver_id: receiver_id.clone(), message_id: message_id.clone(), message_instance_id, _wait_mode: wait_mode, _started_at: Instant::now() })?;

        Ok(Some(worker_wait_guard))
    }

    /// Return the ID of the worker executing the current MWF task.
    ///
    /// Returns an error when called outside an MWF worker task.
    pub fn get_current_worker_id() -> Result<WorkerID, MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        match current_worker_id() {
            Some(worker_id) => Ok(worker_id),
            None => Err(MWFError::Worker_CurrentContextUnavailable),
        }
    }

    /// Return the parent entity ID of the current worker.
    ///
    /// Returns an error when called outside an MWF worker task.
    pub fn get_current_parent_id() -> Result<EntityID, MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        match current_parent_id() {
            Some(parent_id) => Ok(parent_id),
            None => Err(MWFError::Worker_CurrentContextUnavailable),
        }
    }

    /// Create and register a new worker instance from its WorkerTypeID.
    ///
    /// The worker type must have been previously
    /// registered using MWF::register_worker_type().
    ///
    /// MWF retrieves the registered worker constructor,
    /// creates a new worker instance, then spawns it.
    pub fn spawn_worker<TWorkerTypeID>(worker_type_id: &TWorkerTypeID, parent_id: &EntityID) -> Result<WorkerID, MWFError>
    where
        TWorkerTypeID: Into<WorkerTypeID> + Clone,
    {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        let worker_type_id = worker_type_id.clone().into();

        // Ensure the worker type and parent entity identifiers are valid.
        if !worker_type_id.is_valid() {
            return Err(MWFError::WorkerType_Invalid { worker_type_id });
        }
        if !parent_id.is_valid() {
            return Err(MWFError::Entity_Invalid { entity_id: parent_id.clone() });
        }

        // Ensure the framework has been initialized.
        let mwf_data_guard = MWF_DATA.read().expect(MWFPanic::MWFData_ReadLockPoisoned.message());
        let mwf_data = mwf_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;

        // Ensure the worker type has been registered.
        if !mwf_data.worker_type_registry.contains_worker_type(&worker_type_id) {
            return Err(MWFError::WorkerType_NotRegistered);
        }

        // Retrieve the constructor associated with the WorkerTypeID.
        let worker_constructor = mwf_data.worker_type_registry.get_worker_constructor(&worker_type_id)?;

        // Create a new worker instance.
        let worker = worker_constructor();

        // Register the worker before releasing the MWF data lock.
        // This prevents MWF::close() from passing its shutdown synchronization
        // while a worker is still being created.
        let result = MWFWrk::spawn_worker_instance(worker_type_id, worker, parent_id);

        drop(mwf_data_guard);

        result
    }

    /// Spawn and register a worker instance.
    ///
    /// The worker instance must have been created from a
    /// previously registered WorkerTypeID.
    ///
    /// Steps:
    /// 1. Generate a unique WorkerID.
    /// 2. Create communication channels used by MWF.
    /// 3. Spawn the Tokio task that will own the worker instance.
    /// 4. Register the worker resources in the Registry.
    ///
    /// After this function returns:
    /// - MWF owns the WorkerID, channels and JoinHandle.
    /// - The Tokio task owns the worker instance itself.
    pub(crate) fn spawn_worker_instance(worker_type_id: WorkerTypeID, worker: Box<dyn Worker + Send>, parent_id: &EntityID) -> Result<WorkerID, MWFError> {
        let mut mwf_data_guard = MWF_WRK_DATA.write().expect(MWFPanic::MWFWrkData_WriteLockPoisoned.message());
        let mwf_data = mwf_data_guard.as_mut().ok_or(MWFError::Framework_NotInitialized)?;

        // Child workers can only be attached to an
        // initialized parent worker.
        //
        // This prevents creating new hierarchy branches
        // while the parent is starting up or shutting down.
        match parent_id {
            EntityID::Worker(parent_id_ref) => {
                let worker_state = mwf_data.worker_registry.get_worker_state(parent_id_ref)?;

                if worker_state != WorkerState::Initialized {
                    return Err(MWFError::Worker_NotInitialized);
                }
            }

            EntityID::External(_) => {}
        }

        let worker_id = mwf_data.worker_registry.generate_worker_id(&worker_type_id);

        let action_start_timer = Instant::now();

        // Channel used to send worker commands (like Init and Close) and messages to the worker.
        let (normal_sender_channel, normal_receiver_channel) = tokio::sync::mpsc::channel(mwf_data.worker_normal_channel_capacity);
        // Channel used to send priority commands and messages to the worker.
        let (priority_sender_channel, priority_receiver_channel) = tokio::sync::mpsc::channel(mwf_data.worker_priority_channel_capacity);

        // Spawn the Tokio task that will own the worker instance.
        //
        // Once spawned, MWF no longer owns the worker.
        // MWF communicates with the worker through channels and
        // tracks its execution through the JoinHandle.
        let join_handle = spawn_worker_task(worker, normal_receiver_channel, priority_receiver_channel, worker_id.clone(), parent_id.clone());

        // Store the worker resources in the Registry.
        //
        // The Registry stores:
        // - worker hierarchy information.
        // - worker state.
        // - Communication channels.
        // - JoinHandle associated with the Tokio task.
        match mwf_data.worker_registry.add_worker(&worker_id, &worker_type_id, parent_id, normal_sender_channel, priority_sender_channel, join_handle) {
            Ok(()) => {}

            Err((error, join_handle)) => {
                // Abort the spawned Tokio task to avoid leaving
                // an orphan worker running.
                join_handle.abort();

                MWFLog::add_log(action_start_timer, &worker_id, LogAction::WorkerSpawn, LogStatus::Error)?;

                return Err(error);
            }
        }

        MWFLog::add_log(action_start_timer, &worker_id, LogAction::WorkerSpawn, LogStatus::Ok)?;

        drop(mwf_data_guard);

        Ok(worker_id)
    }

    /// Execute blocking work without blocking the MWF runtime.
    ///
    /// Typical uses include native DLL calls and CPU-intensive computations.
    ///
    /// This is a thin wrapper around [`tokio::task::spawn_blocking`].
    ///
    /// The blocking work is executed on Tokio's blocking thread pool,
    /// allowing the Tokio runtime to continue executing other MWF workers.
    pub async fn spawn_blocking<F, R>(f: F) -> Result<R, MWFError>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        let worker_context = current_worker_context();
        let message_context = current_message_inheritance_context();

        tokio::task::spawn_blocking(move || match worker_context {
            Some(worker_context) => scope_worker_context(worker_context, message_context, f),
            None => f(),
        })
        .await
        .map_err(|e| MWFError::BlockingTask_Failed(anyhow::anyhow!(e)))
    }

    /// Suspend the current worker for the specified duration.
    ///
    /// This is a thin wrapper around [`tokio::time::sleep`].
    ///
    /// While this worker is suspended, the Tokio runtime can continue
    /// executing other MWF workers.
    pub async fn sleep(duration: Duration) {
        tokio::time::sleep(duration).await;
    }

    /// Get the current status of a worker.
    pub fn get_worker_status(worker_id: &WorkerID) -> Result<WorkerStatus, MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        let mwf_wrk_data_guard = MWF::validate_api_context()?;

        let mwf_data = mwf_wrk_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;
        let registry_entry = mwf_data.worker_registry.get_registry_entry(worker_id)?;
        let normal_channel_capacity = mwf_data.worker_normal_channel_capacity;
        let priority_channel_capacity = mwf_data.worker_priority_channel_capacity;
        let normal_channel_size = normal_channel_capacity - registry_entry.normal_sender_channel.capacity();
        let priority_channel_size = priority_channel_capacity - registry_entry.priority_sender_channel.capacity();

        let worker_status = WorkerStatus { worker_state: registry_entry.worker_state, normal_channel_size, normal_channel_capacity, priority_channel_size, priority_channel_capacity };

        Ok(worker_status)
    }

    /// Retrieve information about all workers currently
    /// registered in MWF.
    ///
    /// The returned list contains a snapshot of the Registry
    /// state at the time of the call.
    ///
    /// This function is primarily intended for:
    /// - Debug tools.
    /// - Monitoring interfaces.
    /// - Runtime inspection.
    ///
    /// Returns Framework_NotInitialized if MWF has not
    /// been initialized through MWF::init().
    pub fn get_worker_info_list() -> Result<WorkerList, MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        let mwf_wrk_data_guard = MWF::validate_api_context()?;

        let mwf_data = mwf_wrk_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;

        Ok(WorkerList { worker_info_list: mwf_data.worker_registry.get_worker_info_list() })
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
    /// - Framework_NotInitialized if MWF has not been initialized.
    /// - Worker_NotFound if the worker does not exist.
    pub fn get_worker_type_rust_path(worker_id: &WorkerID) -> Result<&'static str, MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        let mwf_wrk_data_guard = MWF::validate_api_context()?;

        // Get the WorkerTypeID associated with the worker.
        let mwf_wrk_data = mwf_wrk_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;
        let worker_type_id = mwf_wrk_data.worker_registry.get_worker_type_id(worker_id)?;
        drop(mwf_wrk_data_guard);

        // Get the Rust type name associated with the WorkerTypeID.
        let mwf_data_guard = MWF_DATA.read().expect(MWFPanic::MWFData_ReadLockPoisoned.message());
        let mwf_data = mwf_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;
        let worker_rust_type_name = mwf_data.worker_type_registry.get_worker_rust_type_name(&worker_type_id)?;

        Ok(worker_rust_type_name)
    }

    /// Request worker initialization.
    ///
    /// The worker state is immediately changed to Initializing.
    /// This function only queues the Init command and does not wait
    /// for on_init() completion.
    ///
    /// This function cannot be called by a worker to initialize itself.
    pub async fn init_worker<TInitData>(worker_id: &WorkerID, init_data: &TInitData) -> Result<(), MWFError>
    where
        TInitData: Clone + Send + Sync + 'static,
    {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        // A worker cannot initialize itself.
        if current_worker_id().as_ref() == Some(worker_id) {
            return Err(MWFError::Worker_CannotManageSelf);
        }

        // Change worker state to Initializing
        // and read MWF data to retrieve the worker channel
        // associated with the target worker.
        //
        // Only workers in the Created state can be initialized.
        let normal_sender_channel = {
            let mut mwf_data_guard = MWF_WRK_DATA.write().expect(MWFPanic::MWFWrkData_WriteLockPoisoned.message());
            let mwf_data = mwf_data_guard.as_mut().ok_or(MWFError::Framework_NotInitialized)?;
            let worker_state = mwf_data.worker_registry.get_worker_state(worker_id)?;
            if worker_state != WorkerState::Created {
                return Err(MWFError::Worker_AlreadyInitialized);
            }
            mwf_data.worker_registry.set_worker_state(worker_id, WorkerState::Initializing)?;

            mwf_data.worker_registry.get_worker_normal_channel(worker_id)?
        };

        // Send the Init command to WorkerTask.
        normal_sender_channel.send(WorkerCommand::Init { init_data: MessageData::new(init_data) }).await.map_err(|_| MWFError::Worker_ChannelClosed)?;

        Ok(())
    }

    /// Request worker shutdown.
    ///
    /// The worker state is immediately changed to Closing.
    /// This function only queues the Close command and does not wait
    /// for on_close() completion.
    ///
    /// This function cannot be called by a worker to close itself.
    pub async fn close_worker<TCloseData>(worker_id: &WorkerID, close_data: &TCloseData) -> Result<(), MWFError>
    where
        TCloseData: Clone + Send + Sync + 'static,
    {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        // A worker cannot close itself.
        if current_worker_id().as_ref() == Some(worker_id) {
            return Err(MWFError::Worker_CannotManageSelf);
        }

        // Change worker state to Closing
        // and read MWF data to retrieve the worker channel
        // associated with the target worker.
        //
        // Only workers in the Initialized state can be closed.
        let normal_sender_channel = {
            let mut mwf_data_guard = MWF_WRK_DATA.write().expect(MWFPanic::MWFWrkData_WriteLockPoisoned.message());
            let mwf_data = mwf_data_guard.as_mut().ok_or(MWFError::Framework_NotInitialized)?;
            let worker_state = mwf_data.worker_registry.get_worker_state(worker_id)?;
            if worker_state != WorkerState::Initialized {
                return Err(MWFError::Worker_NotInitialized);
            }
            mwf_data.worker_registry.set_worker_state(worker_id, WorkerState::Closing)?;
            mwf_data.worker_registry.get_worker_normal_channel(worker_id)?
        };

        // Send the Close command to WorkerTask.
        normal_sender_channel.send(WorkerCommand::Close { close_data: MessageData::new(close_data) }).await.map_err(|_| MWFError::Worker_ChannelClosed)?;

        Ok(())
    }

    /// Get the ParentID of a worker.
    pub fn get_parent_id(worker_id: &WorkerID) -> Result<EntityID, MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        let mwf_wrk_data_guard = MWF::validate_api_context()?;

        let mwf_data = mwf_wrk_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;
        let parent_id = mwf_data.worker_registry.get_parent_id(worker_id)?;

        Ok(parent_id)
    }

    /// Get the child WorkerIDs of an entity.
    pub fn get_children_ids(entity_id: &EntityID) -> Result<Vec<WorkerID>, MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        let mwf_wrk_data_guard = MWF::validate_api_context()?;

        // Validate the entity identifier.
        if !entity_id.is_valid() {
            return Err(MWFError::Entity_Invalid { entity_id: entity_id.clone() });
        }
        let mwf_data = mwf_wrk_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;

        mwf_data.worker_registry.get_children_ids(entity_id)
    }

    /// Get the number of child workers of an entity.
    pub fn get_children_count(entity_id: &EntityID) -> Result<usize, MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        let mwf_wrk_data_guard = MWF::validate_api_context()?;

        // Validate the entity identifier.
        if !entity_id.is_valid() {
            return Err(MWFError::Entity_Invalid { entity_id: entity_id.clone() });
        }
        let mwf_data = mwf_wrk_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;
        let children_count = mwf_data.worker_registry.get_children_count(entity_id)?;
        drop(mwf_wrk_data_guard);
        Ok(children_count)
    }

    /// Wait until all workers in the list reach the Initialized state.
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    ///
    /// A worker cannot wait for its own initialization.
    pub async fn wait_worker_list_init(worker_ids: &Vec<WorkerID>, timeout_ms: Option<u64>) -> Result<(), MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        // A worker cannot wait for its own initialization.
        if let Some(current_id) = current_worker_id() {
            if worker_ids.contains(&current_id) {
                return Err(MWFError::Worker_CannotWaitForSelf);
            }
        }

        let wait_future = async {
            loop {
                // Register interest in the next initialization event
                // before checking worker states to avoid lost notifications.
                let init_notify = MWF_WRK_INIT_NOTIFY.notified();
                tokio::pin!(init_notify);
                init_notify.as_mut().enable();

                // Read Registry state under a short-lived lock.
                //
                // The lock must be released before awaiting the
                // next notification.
                let (all_init_finished, one_init_failed) = {
                    let mwf_wrk_data_guard = MWF_WRK_DATA.read().expect(MWFPanic::MWFWrkData_ReadLockPoisoned.message());
                    let mwf_data = mwf_wrk_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;

                    let mut all_init_finished = true;
                    let mut one_init_failed = false;

                    // Initialization is complete only when all workers
                    // have reached the Initialized or InitFailed state.
                    for worker_id in worker_ids {
                        match mwf_data.worker_registry.get_worker_state(worker_id)? {
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
                        return Err(MWFError::Worker_InitializationFailed);
                    }

                    return Ok(());
                }

                // Wait for the next worker initialization event.
                init_notify.await;
            }
        };

        match timeout_ms {
            Some(timeout_ms) => tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), wait_future).await.map_err(|_| MWFError::Operation_Timeout)?,
            None => wait_future.await,
        }
    }

    /// Wait until a worker reaches the Initialized state.
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    pub async fn wait_worker_init(worker_id: &WorkerID, timeout_ms: Option<u64>) -> Result<(), MWFError> {
        MWFWrk::wait_worker_list_init(&vec![worker_id.to_owned()], timeout_ms).await
    }

    /// Wait until all direct children of an entity reach the
    /// Initialized state.
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    pub async fn wait_children_init(entity_id: &EntityID, timeout_ms: Option<u64>) -> Result<(), MWFError> {
        // Resolve the current child list and delegate the wait
        // operation to wait_worker_list_init().
        MWFWrk::wait_worker_list_init(&MWFWrk::get_children_ids(entity_id)?, timeout_ms).await
    }

    /// Wait until all workers in the list are no longer registered in MWF.
    ///
    /// A worker is considered finished when it has been removed from the Registry.
    /// This normally happens after on_close() completes, but can also happen
    /// following a worker abort or panic.
    ///
    /// During a normal shutdown, the worker task completes its lifecycle
    /// before being removed from the Registry.
    ///
    /// If the worker is aborted, it is removed from the Registry without
    /// waiting for its Tokio task to terminate. Tasks blocked in synchronous
    /// code or native DLL calls may continue executing.
    ///
    /// Returns Worker_CloseFailed if a worker failed during on_close().
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    ///
    /// A worker cannot wait for its own closure.
    pub async fn wait_worker_list_close(worker_ids: &Vec<WorkerID>, timeout_ms: Option<u64>) -> Result<(), MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        // A worker cannot wait for its own closure.
        if let Some(current_id) = current_worker_id() {
            if worker_ids.contains(&current_id) {
                return Err(MWFError::Worker_CannotWaitForSelf);
            }
        }

        let wait_future = async {
            loop {
                // Register interest in the next closing event
                // before checking worker states to avoid lost notifications.
                let close_notify = MWF_WRK_CLOSE_NOTIFY.notified();
                tokio::pin!(close_notify);
                close_notify.as_mut().enable();

                // Read Registry state under a short-lived lock.
                //
                // The lock must be released before awaiting the
                // next notification.
                let (all_close_finished, one_close_failed) = {
                    let mwf_wrk_data_guard = MWF_WRK_DATA.read().expect(MWFPanic::MWFWrkData_ReadLockPoisoned.message());
                    let mwf_data = mwf_wrk_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;

                    let mut all_close_finished = true;
                    let mut one_close_failed = false;

                    // Closing is complete only when all workers
                    // have either been removed from the Registry
                    // or reached the CloseFailed or the Panicked state.
                    for worker_id in worker_ids {
                        match mwf_data.worker_registry.get_worker_state(worker_id) {
                            // A worker not found has been properly closed.
                            Err(MWFError::Worker_NotFound) => {}

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
                        return Err(MWFError::Worker_CloseFailed);
                    }

                    return Ok(());
                }

                close_notify.await;
            }
        };

        match timeout_ms {
            Some(timeout_ms) => tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), wait_future).await.map_err(|_| MWFError::Operation_Timeout)?,
            None => wait_future.await,
        }
    }

    /// Wait until a worker is no longer registered in MWF.
    ///
    /// See wait_worker_list_close() for details.
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    pub async fn wait_worker_close(worker_id: &WorkerID, timeout_ms: Option<u64>) -> Result<(), MWFError> {
        MWFWrk::wait_worker_list_close(&vec![worker_id.to_owned()], timeout_ms).await
    }

    /// Wait until all direct children of an entity are no longer registered in MWF.
    ///
    /// See wait_worker_list_close() for details.
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    pub async fn wait_children_close(entity_id: &EntityID, timeout_ms: Option<u64>) -> Result<(), MWFError> {
        // Resolve the current child list and delegate the wait
        // operation to wait_worker_list_close().
        MWFWrk::wait_worker_list_close(&MWFWrk::get_children_ids(entity_id)?, timeout_ms).await
    }

    /// Aborts a list of workers.
    ///
    /// This is a best-effort operation intended as a last resort when workers cannot be closed
    /// normally. It requests immediate task cancellation through Tokio when possible and removes
    /// each worker from the Registry, even if its JoinHandle cannot be retrieved or task
    /// cancellation cannot complete.
    ///
    /// It does not wait for task termination. Tasks blocked in synchronous code or native DLL
    /// calls may continue executing.
    ///
    /// MWF-managed resources are cleaned up as far as possible, but resources owned by code that
    /// remains blocked cannot be guaranteed to be released.
    ///
    /// Any worker-related error is recorded, but does not prevent the remaining workers from
    /// being processed. The first encountered error is returned after all workers have been processed.
    ///
    /// Aborted workers that continue executing may still emit log events, even after MWF has been
    /// reinitialized. Such events retain their original WorkerID and can be identified as belonging
    /// to a previous MWF instance.
    pub fn abort_worker_list(worker_ids: &Vec<WorkerID>) -> Result<(), MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        let mut first_error: Option<MWFError> = None;
        let mut worker_action_list = Vec::with_capacity(worker_ids.len());

        let mut mwf_data_guard = MWF_WRK_DATA.write().expect(MWFPanic::MWFWrkData_WriteLockPoisoned.message());
        let mwf_data = mwf_data_guard.as_mut().ok_or(MWFError::Framework_NotInitialized)?;

        // Request cancellation of all worker tasks.
        for worker_id in worker_ids {
            let action_start_timer = Instant::now();

            match mwf_data.worker_registry.take_join_handle(worker_id) {
                Ok(join_handle) => {
                    // Request immediate cancellation without waiting for task termination.
                    join_handle.abort();
                }
                Err(error) => {
                    // Keep processing the remaining workers.
                    let _ = MWFLog::add_log(action_start_timer, worker_id, LogAction::WorkerAbort, LogStatus::Error);

                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
            }

            worker_action_list.push((worker_id.clone(), action_start_timer));
        }

        // Remove all targeted workers from the Registry, regardless of cancellation results.
        for (worker_id, action_start_timer) in &worker_action_list {
            match mwf_data.worker_registry.remove_worker(worker_id) {
                Ok(()) => {
                    let _ = MWFLog::add_log(*action_start_timer, worker_id, LogAction::WorkerAbort, LogStatus::Ok);
                    let _ = MWFLog::remove_logged_worker(worker_id);
                }
                Err(error) => {
                    let _ = MWFLog::add_log(*action_start_timer, worker_id, LogAction::WorkerAbort, LogStatus::Error);

                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
            }
        }

        drop(mwf_data_guard);

        // Wake up all waiters interested in worker initialization and shutdown events.
        MWF_WRK_CLOSE_NOTIFY.notify_waiters();
        MWF_WRK_INIT_NOTIFY.notify_waiters();

        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    /// Aborts a worker.
    ///
    /// This is a best-effort operation intended as a last resort when a worker cannot be closed
    /// normally.
    ///
    /// See abort_worker_list() for details.
    pub fn abort_worker(worker_id: &WorkerID) -> Result<(), MWFError> {
        MWFWrk::abort_worker_list(&vec![worker_id.to_owned()])
    }

    /// Aborts all children of an entity.
    ///
    /// This is a best-effort operation intended as a last resort when a workers cannot be closed
    /// normally.
    ///
    /// See abort_worker_list() for details.
    pub fn abort_children(entity_id: &EntityID) -> Result<(), MWFError> {
        MWFWrk::abort_worker_list(&MWFWrk::get_children_ids(entity_id)?)
    }

    /// Send a message through the specified worker channel.
    async fn send_message_internal<TMessageID, TMessageData>(sender_id: &EntityID, receiver_id: &WorkerID, message_id: &TMessageID, message_data: &TMessageData, channel: WorkerChannel) -> Result<(), MWFError>
    where
        TMessageID: Into<MessageID> + Clone,
        TMessageData: Clone + Send + Sync + 'static,
    {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        // Validate the sender entity identifier.
        if !sender_id.is_valid() {
            return Err(MWFError::Entity_Invalid { entity_id: sender_id.clone() });
        }

        // Validate the message identifier.
        let message_id = message_id.clone().into();
        if !message_id.is_valid() {
            return Err(MWFError::Message_Invalid { message_id });
        }

        // Determine the message priority and retrieve its sender channel.
        let priority = match &channel {
            WorkerChannel::Normal => false,
            WorkerChannel::Priority => true,
        };

        // Generate a unique identifier for this message instance.
        let message_instance_id = Self::generate_message_instance_id()?;

        // Inherit the current message flow only when sending to the same worker.
        let inheritance_context = Self::get_message_inheritance_context_for_receiver(receiver_id);

        let framework_message = WorkerMessage::new(sender_id, receiver_id, &message_id, message_instance_id, message_data, inheritance_context);

        // Send the message to the target worker.
        send_worker_command(sender_id, receiver_id, channel, WorkerCommand::Message { message: framework_message, priority, unique: false }).await
    }

    /// Send a message and wait for the requested completion mode.
    async fn send_message_and_wait_internal<TMessageID, TMessageData>(sender_id: &EntityID, receiver_id: &WorkerID, message_id: &TMessageID, wait_mode: MessageWaitMode, message_data: &TMessageData, timeout_ms: Option<u64>, channel: WorkerChannel) -> Result<(), MWFError>
    where
        TMessageID: Into<MessageID> + Clone,
        TMessageData: Clone + Send + Sync + 'static,
    {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        // Validate the sender entity identifier.
        if !sender_id.is_valid() {
            return Err(MWFError::Entity_Invalid { entity_id: sender_id.clone() });
        }

        // Validate the message identifier.
        let message_id = message_id.clone().into();
        if !message_id.is_valid() {
            return Err(MWFError::Message_Invalid { message_id });
        }

        // Resolve the channel and whether the message is a priority message.
        let priority = match &channel {
            WorkerChannel::Normal => false,
            WorkerChannel::Priority => true,
        };

        // Generate an identifier for this message instance.
        let message_instance_id = Self::generate_message_instance_id()?;

        // Register the current worker's wait to detect deadlocks.
        let _worker_wait_guard = Self::register_current_worker_wait(receiver_id, &message_id, message_instance_id, wait_mode)?;

        match wait_mode {
            MessageWaitMode::WaitThisMessage => wait_this_message(sender_id, receiver_id, &message_id, message_instance_id, message_data, timeout_ms, priority, channel).await,
            MessageWaitMode::WaitThisMessageFlow => {
                let message_flow_id = Self::generate_message_flow_id()?;

                wait_message_flow(sender_id, receiver_id, &message_id, message_instance_id, message_flow_id, message_data, timeout_ms, priority, channel).await
            }
        }
    }

    /// Send a message to a worker.
    ///
    /// `message_id` may be any type implementing `Into<MessageID>`.
    pub async fn send_message<TMessageID, TMessageData>(sender_id: &EntityID, receiver_id: &WorkerID, message_id: &TMessageID, message_data: &TMessageData) -> Result<(), MWFError>
    where
        TMessageID: Into<MessageID> + Clone,
        TMessageData: Clone + Send + Sync + 'static,
    {
        Self::send_message_internal(sender_id, receiver_id, message_id, message_data, WorkerChannel::Normal).await
    }

    /// Send a message and wait for the requested completion mode.
    ///
    /// `message_id` may be any type implementing `Into<MessageID>`.
    ///
    /// The completion mode determines what the caller waits for:
    /// - `WaitThisMessage`: completion of the receiver's `on_message()` callback.
    /// - `WaitThisMessageFlow`: explicit completion of the message flow through `message_flow_end()`.
    ///
    /// Returns:
    /// - Ok(()) if the requested completion succeeds.
    /// - An MWFError if the callback or message flow returns an error,
    ///   the worker terminates, or the operation times out.
    pub async fn send_message_and_wait<TMessageID, TMessageData>(sender_id: &EntityID, receiver_id: &WorkerID, message_id: &TMessageID, wait_mode: MessageWaitMode, message_data: &TMessageData, timeout_ms: Option<u64>) -> Result<(), MWFError>
    where
        TMessageID: Into<MessageID> + Clone,
        TMessageData: Clone + Send + Sync + 'static,
    {
        Self::send_message_and_wait_internal(sender_id, receiver_id, message_id, wait_mode, message_data, timeout_ms, WorkerChannel::Normal).await
    }

    /// Send a priority message to a worker.
    ///
    /// Priority messages are always processed before pending
    /// normal messages.
    ///
    /// `message_id` may be any type implementing `Into<MessageID>`.
    pub async fn send_priority_message<TMessageID, TMessageData>(sender_id: &EntityID, receiver_id: &WorkerID, message_id: &TMessageID, message_data: &TMessageData) -> Result<(), MWFError>
    where
        TMessageID: Into<MessageID> + Clone,
        TMessageData: Clone + Send + Sync + 'static,
    {
        Self::send_message_internal(sender_id, receiver_id, message_id, message_data, WorkerChannel::Priority).await
    }

    /// Send a priority message and wait for the requested completion mode.
    ///
    /// Priority messages are always processed before pending
    /// normal messages.
    ///
    /// `message_id` may be any type implementing `Into<MessageID>`.
    ///
    /// The completion mode determines what the caller waits for:
    /// - `WaitThisMessage`: completion of the receiver's `on_message()` callback.
    /// - `WaitThisMessageFlow`: explicit completion of the message flow through `message_flow_end()`.
    ///
    /// Returns:
    /// - Ok(()) if the requested completion succeeds.
    /// - An MWFError if the callback or message flow returns an error,
    ///   the worker terminates, or the operation times out.
    pub async fn send_priority_message_and_wait<TMessageID, TMessageData>(sender_id: &EntityID, receiver_id: &WorkerID, message_id: &TMessageID, wait_mode: MessageWaitMode, message_data: &TMessageData, timeout_ms: Option<u64>) -> Result<(), MWFError>
    where
        TMessageID: Into<MessageID> + Clone,
        TMessageData: Clone + Send + Sync + 'static,
    {
        Self::send_message_and_wait_internal(sender_id, receiver_id, message_id, wait_mode, message_data, timeout_ms, WorkerChannel::Priority).await
    }

    /// Send a unique message to a worker.
    ///
    /// At most one unique message with the same MessageID
    /// is retained in the worker's pending command queue.
    ///
    /// Sending a new unique message replaces the pending one while
    /// preserving its FIFO position.
    ///
    /// `message_id` may be any type implementing `Into<MessageID>`.
    pub async fn send_unique_message<TMessageID, TMessageData>(sender_id: &EntityID, receiver_id: &WorkerID, message_id: &TMessageID, message_data: &TMessageData) -> Result<(), MWFError>
    where
        TMessageID: Into<MessageID> + Clone,
        TMessageData: Clone + Send + Sync + 'static,
    {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        // Validate the entity identifier.
        if !sender_id.is_valid() {
            return Err(MWFError::Entity_Invalid { entity_id: sender_id.clone() });
        }
        // Convert the message identifier.
        let message_id = message_id.clone().into();
        if !message_id.is_valid() {
            return Err(MWFError::Message_Invalid { message_id });
        }

        // Generate the unique id and build the message
        let message_instance_id = MWFWrk::generate_message_instance_id()?;

        // Inherit the current message flow only when sending a message to this same worker.
        let inheritance_context = MWFWrk::get_message_inheritance_context_for_receiver(receiver_id);

        let framework_message = WorkerMessage::new(sender_id, receiver_id, &message_id, message_instance_id, message_data, inheritance_context);

        // Send the unique message to the target worker.
        send_worker_command(sender_id, receiver_id, WorkerChannel::Normal, WorkerCommand::Message { message: framework_message, priority: false, unique: true }).await
    }

    /// Complete the message flow associated with the current message and forward its result.
    ///
    /// `message flow_result` is returned by the caller waiting for the message flow.
    ///
    /// The message flow result represents the outcome of the application-level processing.
    /// The returned MWFError only indicates that the framework could not signal completion.
    ///
    /// Returns:
    /// - Worker_CurrentContextUnavailable if called outside an MWF worker task.
    /// - MessageFlow_NotActive if the current message has no active flow.
    pub fn message_flow_end(message_flow_result: Result<(), WorkerError>) -> Result<(), MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        // Identify the worker executing the current callback.
        let worker_id = current_worker_id().ok_or(MWFError::Worker_CurrentContextUnavailable)?;

        // Retrieve the message flow associated with the currently processed message.
        let message_flow_id = current_message_inheritance_context().message_flow_id.ok_or(MWFError::MessageFlow_NotActive)?;

        // Remove the flow and forward its result to the waiting caller.
        complete_message_flow(message_flow_id, &worker_id, message_flow_result)
    }
}
