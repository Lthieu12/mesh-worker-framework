#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
mod channel;
mod framework_error;
mod log_file;
mod message;
mod message_wait;
mod string_helper;
mod worker;
mod worker_registry;
mod worker_task;
mod worker_type_registry;

mod mwf;
mod mwf_err;
mod mwf_log;
mod mwf_wrk;

// only used for tests, examples and debug.
//
// It is not considered part of the stable public API
// and may change between releases.
#[doc(hidden)]
pub mod dev_support;

// Public MWF API
pub use mwf::{MWF, MWFConfiguration};
pub use mwf_err::{ErrorEvent, ErrorInstanceID, ErrorList, ErrorOutputFormat, MWFErr};
pub use mwf_log::{LogAction, LogEvent, LogInstanceID, LogList, LogMode, LogOutputFormat, LogStatus, MWFLog};
pub use mwf_wrk::MWFWrk;

pub use framework_error::MWFError;

pub use worker::WorkerError;

pub use worker::{EntityID, ExternalID, Worker, WorkerID, WorkerInfo, WorkerList, WorkerOutputFormat, WorkerState, WorkerStatus, WorkerTypeID};

pub use message::{MessageData, MessageID, WorkerMessage};
