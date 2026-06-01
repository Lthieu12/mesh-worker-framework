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


pub mod error;
pub mod module;
pub mod message;
pub mod registry;
pub mod mmf;
pub mod logging;

