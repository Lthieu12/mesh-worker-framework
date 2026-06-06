//! Framework error definitions.
//!
//! This module contains all errors that can be returned by MMF public APIs.

use std::error::Error;
use std::fmt;

/// Errors returned by MMF functions.
///
/// Naming convention:
/// - Category_Error
/// - Examples:
///   - Framework_NotInitialized
///   - Module_NotFound
///   - Module_ChannelClosed
#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum FrameworkError {

    /// The framework has not been initialized.
    Framework_NotInitialized,

    /// The framework is already initialized.
    Framework_AlreadyInitialized,

    /// A module failed during its initialization.
    Module_InitializationFailed,

    /// The requested module was not found.
    Module_NotFound,

    /// A module communication channel is closed.
    Module_ChannelClosed,

    /// The requested module already exists.
    Module_AlreadyExists,

    /// The requested module is already initialized.
    Module_AlreadyInitialized
}

/// Human-readable representation of FrameworkError.
impl fmt::Display for FrameworkError {

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {

        match self {

            FrameworkError::Framework_NotInitialized =>
                write!(f, "The framework has not been initialized."),

            FrameworkError::Framework_AlreadyInitialized =>
                write!(f, "The framework is already initialized."),

            FrameworkError::Module_InitializationFailed =>
                write!(f, "A module failed during its initialization."),

            FrameworkError::Module_NotFound =>
                write!(f, "The requested module was not found."),

            FrameworkError::Module_ChannelClosed =>
                write!(f, "A module communication channel is closed."),

            FrameworkError::Module_AlreadyExists =>
                write!(f, "The requested module already exists."),
            
            FrameworkError::Module_AlreadyInitialized =>
                write!(f, "The requested module is already initialized."),

        }
    }
}

/// Standard Rust error implementation for FrameworkError.
impl Error for FrameworkError {}