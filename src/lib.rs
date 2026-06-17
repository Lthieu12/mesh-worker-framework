//! Mesh Module Framework (MMF).
//!
//! MMF is a lightweight messaging framework designed to build
//! modular and hierarchical applications.
//!
//! The framework provides:
//! - Module lifecycle management
//! - Message-based communication
//! - Parent/child module hierarchy
//! - Centralized logging
//! - Request/reply messaging patterns
//!
//! Applications interact with the framework through the MMF API.
mod registry;
mod channel;
mod module_task;
mod logging;
mod error;
mod module;
mod message;

pub mod mmf;
pub mod mmf_mod;
pub mod mmf_log;


// Public MMF API
pub use mmf::MMF; 
pub use mmf_mod::MMFMod;
pub use mmf_log::MMFLog;

pub use error::FrameworkError;

pub use mmf_mod::ModuleError;

pub use module::{
    Module,
    ModuleID,
    EntityID,
    ModuleState,
    ModuleStatus,
};
pub use message::{
    FrameworkMessage,
    MessageID,
};

