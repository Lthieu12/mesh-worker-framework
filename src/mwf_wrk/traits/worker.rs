use async_trait::async_trait;

use crate::mwf_wrk::types::{MessageData, WorkerError, WorkerMessage, WorkerTypeID};

/// Interface implemented by all MWF workers.
///
/// MWF creates worker instances dynamically by calling
/// `worker::new()` on the registered worker type.
///
/// MWF stores worker instances behind `Box<dyn Worker>`.
///
/// The `Self: Sized` bounds on `worker_type_id()` and `new()` are
/// required to keep this trait object-safe. Without these bounds,
/// `worker` could no longer be used as `dyn Worker`.
///
#[async_trait]
pub trait Worker {
    /// Returns the unique identifier of this worker type.
    ///
    /// Every worker implementation must provide a unique WorkerTypeID.
    ///
    /// This associated function is only available on concrete worker
    /// types (`Self: Sized`). It is intentionally excluded from trait
    /// objects (`dyn Worker`) so that MWF can store workers behind
    /// `Box<dyn Worker>`.
    fn worker_type_id() -> WorkerTypeID
    where
        Self: Sized;

    /// Creates a new worker instance.
    ///
    /// MWF calls this function whenever a new worker
    /// instance must be spawned from its registered
    /// WorkerTypeID.
    ///
    /// The returned instance must be in its initial state.
    /// Runtime resources should be acquired later during
    /// on_init().
    ///
    /// This associated function is only available on concrete worker
    /// types (`Self: Sized`). It is intentionally excluded from trait
    /// objects (`dyn worker`) so that MWF can store workers behind
    /// `Box<dyn worker>`.
    fn new() -> Self
    where
        Self: Sized;

    /// Called once when the worker is initialized.
    async fn on_init(&mut self, init_data: MessageData) -> Result<(), WorkerError>;

    /// Called when a message is received by the worker.
    async fn on_message(&mut self, message: WorkerMessage) -> Result<(), WorkerError>;

    /// Called once before the worker is closed.
    async fn on_close(&mut self, close_data: MessageData) -> Result<(), WorkerError>;
}
