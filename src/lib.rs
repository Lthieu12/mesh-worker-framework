#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![allow(clippy::expect_fun_call)] // `expect()` keeps MMFPanic messages concise and readable.
#![allow(clippy::single_match)] // Keep match statements ready for future behavior extensions.
mod module_registry;
mod module_type_registry;
mod channel;
mod module_task;
mod framework_error;
mod module;
mod message;
mod string_helper;

pub mod mmf;
pub mod mmf_mod;
pub mod mmf_log;
pub mod mmf_err;


// only used for tests, examples and debug.
// 
// It is not considered part of the stable public API
// and may change between releases.
#[doc(hidden)]
pub mod dev_support;

// Public MMF API
pub use mmf::{MMF, MMFConfiguration}; 
pub use mmf_mod::MMFMod;
pub use mmf_log::{MMFLog, LogInstanceID, LogAction, LogStatus, LogMode, LogEvent, LogList, LogOutputFormat};
pub use mmf_err::{MMFErr, ErrorInstanceID, ErrorEvent, ErrorList, ErrorOutputFormat};

pub use framework_error::FrameworkError;

pub use module::ModuleError;

pub use module::{
    Module,
    ModuleTypeID,
    ModuleID,
    EntityID,
    ModuleState,
    ModuleStatus,
    ModuleInfo,
    ModuleList,
    ModuleOutputFormat,
};

pub use message::{
    ModuleMessage,
    MessageID,
};

