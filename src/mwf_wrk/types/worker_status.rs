use crate::mwf_wrk::types::WorkerState;

/// Current status of a worker within the framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerStatus {
    /// Current lifecycle state of the worker.
    pub worker_state: WorkerState,
    /// Number of normal messages currently waiting in the worker queue.
    pub normal_channel_size: usize,
    /// Maximum number of normal messages that can be queued by the worker.
    pub normal_channel_capacity: usize,
    /// Number of priority messages currently waiting in the worker queue.
    pub priority_channel_size: usize,
    /// Maximum number of priority messages that can be queued by the worker.
    pub priority_channel_capacity: usize,
}
