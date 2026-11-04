#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
mod channel;
mod log_file;
mod string_helper;
mod wait_registry;
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
pub use mwf::{MWF, MWFConfiguration, MWFInfo, TimestampFormat};
pub use mwf_err::{ErrorEvent, ErrorInstanceID, ErrorList, ErrorOutputFormat, MWFErr};
pub use mwf_log::{LogAction, LogEvent, LogInstanceID, LogList, LogMode, LogOutputFormat, LogStatus, MWFLog};
pub use mwf_wrk::MWFWrk;

pub use mwf::MWFError;

pub use mwf_wrk::Worker;
pub use mwf_wrk::types::{EntityID, ExternalID, WorkerError, WorkerID, WorkerInfo, WorkerList, WorkerOutputFormat, WorkerState, WorkerStatus, WorkerTypeID};

pub use mwf_wrk::types::{MessageData, MessageID, MessageWaitMode, WorkerMessage};
