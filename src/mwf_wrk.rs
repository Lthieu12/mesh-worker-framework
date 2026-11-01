//! Public MWFWrk (worker and messaging) API.
//!
//! This API exposes the framework entry points used to
//! manage workers and send messages.

mod functions;
mod types;

pub(crate) use types::WorkerData;

pub use types::MWFWrk;
