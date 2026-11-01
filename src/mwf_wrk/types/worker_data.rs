use crate::mwf::{MWFConfiguration, MWFInstanceID};
use crate::worker_registry::Registry;

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
