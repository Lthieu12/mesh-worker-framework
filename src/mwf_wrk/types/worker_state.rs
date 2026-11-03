/// Current state of a worker within the framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerState {
    /// worker has been created but not initialized.
    Created,
    /// worker initialization is in progress.
    Initializing,
    /// worker initialization has failed.
    InitFailed,
    /// worker initialization has completed successfully.
    Initialized,
    /// worker Closing is in progress.
    Closing,
    /// worker close has failed.
    CloseFailed,
    /// worker has been closed and is not usable anymore.
    Closed,
    /// worker has panicked and is not usable anymore.
    Panicked,
}

impl WorkerState {
    /// Returns a human-readable representation of the worker state.
    fn as_str(&self) -> &'static str {
        match self {
            WorkerState::Created => "Created",
            WorkerState::Initializing => "Initializing",
            WorkerState::InitFailed => "InitFailed",
            WorkerState::Initialized => "Initialized",
            WorkerState::Closing => "Closing",
            WorkerState::CloseFailed => "CloseFailed",
            WorkerState::Closed => "Closed",
            WorkerState::Panicked => "Panicked",
        }
    }
}

#[path = "worker_info.rs"]
mod worker_info;

pub use worker_info::{WorkerInfo, WorkerList, WorkerOutputFormat};
