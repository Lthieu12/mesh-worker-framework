#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
mod registry;
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


// Public MMF API
pub use mmf::{MMF, MMFConfiguration}; 
pub use mmf_mod::MMFMod;
pub use mmf_log::{MMFLog, LogInstanceID, LogAction, LogStatus, LogMode, LogEvent, LogList};
pub use mmf_err::{MMFErr, ErrorInstanceID, ErrorEvent, ErrorList};

pub use framework_error::FrameworkError;

pub use module::ModuleError;

pub use module::{
    Module,
    ModuleID,
    EntityID,
    ModuleState,
    ModuleStatus,
    ModuleInfo,
    ModuleList,
};

pub use message::{
    FrameworkMessage,
    MessageID,
};

