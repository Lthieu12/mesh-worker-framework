#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![allow(clippy::expect_fun_call)] // `expect()` keeps MWFPanic messages concise and readable.
#![allow(clippy::single_match)] // Keep match statements ready for future behavior extensions.
#![allow(clippy::collapsible_if)] // Preserve explicit multi-step control flow for readability.
mod channel;
mod framework_error;
mod log_file;
mod message;
mod string_helper;
mod worker;
mod worker_registry;
mod worker_task;
mod worker_type_registry;

pub mod mmf;
pub mod mmf_err;
pub mod mmf_log;
pub mod mmf_wrk;

// only used for tests, examples and debug.
//
// It is not considered part of the stable public API
// and may change between releases.
#[doc(hidden)]
pub mod dev_support;

// Public MWF API
pub use mmf::{MWF, MWFConfiguration};
pub use mmf_err::{ErrorEvent, ErrorInstanceID, ErrorList, ErrorOutputFormat, MWFErr};
pub use mmf_log::{LogAction, LogEvent, LogInstanceID, LogList, LogMode, LogOutputFormat, LogStatus, MWFLog};
pub use mmf_wrk::MWFWrk;

pub use framework_error::MWFError;

pub use worker::WorkerError;

pub use worker::{EntityID, ExternalID, Worker, WorkerID, WorkerInfo, WorkerList, WorkerOutputFormat, WorkerState, WorkerStatus, WorkerTypeID};

pub use message::{MessageID, WorkerMessage};
