//! Public MWFWrk (worker and messaging) API.
//!
//! This API exposes the framework entry points used to
//! manage workers and send messages.

pub(crate) mod functions;
pub(crate) mod traits;
pub mod types;

pub(crate) use types::WorkerData;

pub use traits::Worker;
pub use types::MWFWrk;
