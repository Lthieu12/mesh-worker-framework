use super::super::MWFWrk;
use crate::framework_error::MWFError;
use crate::mwf::MWF;
use crate::worker::WorkerID;
use crate::worker::WorkerStatus;

impl MWFWrk {
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
}
