use super::super::MWFWrk;
use crate::MWFLog;
use crate::mwf::MWF;
use crate::mwf::MWF_DATA;
use crate::mwf::MWF_WRK_DATA;
use crate::mwf::MWFError;
use crate::mwf::MWFPanic;
use crate::mwf_log::LogAction;
use crate::mwf_log::LogStatus;
use crate::mwf_wrk::traits::Worker;
use crate::mwf_wrk::types::EntityID;
use crate::mwf_wrk::types::WorkerID;
use crate::mwf_wrk::types::WorkerState;
use crate::mwf_wrk::types::WorkerTypeID;
use crate::worker_task::spawn_worker_task;
use std::time::Instant;

impl MWFWrk {
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
    fn spawn_worker_instance(worker_type_id: WorkerTypeID, worker: Box<dyn Worker + Send>, parent_id: &EntityID) -> Result<WorkerID, MWFError> {
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

                MWFLog::log_worker_event(action_start_timer, &worker_id, LogAction::WorkerSpawn, LogStatus::Error);

                return Err(error);
            }
        }

        MWFLog::log_worker_event(action_start_timer, &worker_id, LogAction::WorkerSpawn, LogStatus::Ok);

        drop(mwf_data_guard);

        Ok(worker_id)
    }
}
