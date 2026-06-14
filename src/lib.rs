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

pub mod mmf;
pub mod mmf_mod;
pub mod mmf_log;
pub mod logging;
pub mod error;
pub mod module;
pub mod message;

// Public MMF API
pub use error::FrameworkError;

pub use mmf::MMF; 
pub use mmf_mod::MMFMod;
pub use mmf_log::MMFLog;

pub use module::Module;
pub use module::ModuleID;
pub use module::EntityID;
pub use module::ModuleState;

pub use message::FrameworkMessage;
pub use message::MessageID;






