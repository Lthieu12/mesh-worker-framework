//! Worker constructor type alias.

use crate::mwf_wrk::traits::Worker;

/// Function creating a new worker instance.
///
/// Each registered worker type provides a constructor
/// that MWF stores in the WorkerTypeRegistry.
///
/// Whenever MWF needs to spawn a worker from its
/// WorkerTypeID (for example when executing a message flow),
/// this constructor is invoked to create a fresh worker
/// instance.
///
/// The returned instance is then owned by the spawned
/// worker task.
pub(crate) type WorkerConstructor = fn() -> Box<dyn Worker + Send>;
