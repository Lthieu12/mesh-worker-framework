#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![allow(clippy::expect_fun_call)] // `expect()` keeps MMFPanic messages concise and readable.
#![allow(clippy::single_match)] // Keep match statements ready for future behavior extensions.
mod channel;
mod framework_error;
mod message;
mod module;
mod module_registry;
mod module_task;
mod module_type_registry;
mod string_helper;

pub mod mmf;
pub mod mmf_err;
pub mod mmf_log;
pub mod mmf_mod;

// only used for tests, examples and debug.
//
// It is not considered part of the stable public API
// and may change between releases.
#[doc(hidden)]
pub mod dev_support;

// Public MMF API
pub use mmf::{MMF, MMFConfiguration};
pub use mmf_err::{ErrorEvent, ErrorInstanceID, ErrorList, ErrorOutputFormat, MMFErr};
pub use mmf_log::{LogAction, LogEvent, LogInstanceID, LogList, LogMode, LogOutputFormat, LogStatus, MMFLog};
pub use mmf_mod::MMFMod;

pub use framework_error::FrameworkError;

pub use module::ModuleError;

pub use module::{EntityID, Module, ModuleID, ModuleInfo, ModuleList, ModuleOutputFormat, ModuleState, ModuleStatus, ModuleTypeID};

pub use message::{MessageID, ModuleMessage};
