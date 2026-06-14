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

    /// An internal framework error occurred.
    Framework_InternalError,

    /// The operation timed out.
    Operation_Timeout,

    /// The framework has not been initialized.
    Framework_NotInitialized,

    /// The framework is already initialized.
    Framework_AlreadyInitialized,

    /// The framework shutdown failed: one or more modules are still running.
    Framework_CannotClose_ModulesStillRunning,

    /// A module failed during its initialization.
    Module_InitializationFailed,

    /// The requested module was not found.
    Module_NotFound,

    /// A module failed during its closing.
    Module_CloseFailed,

    /// A module communication channel is closed.
    Module_ChannelClosed,

    /// The requested module already exists.
    Module_AlreadyExists,

    /// The requested module is already initialized.
    Module_AlreadyInitialized,

    /// The requested module has not been initialized.
    Module_NotInitialized,

    /// Error returned by a module while processing
    /// a message sent through send_message_and_wait_reply().
    Module_MessageError(
        Box<dyn std::error::Error + Send + Sync>
    ),

    /// The requested father module was not found.
    FatherModule_NotFound,
}

/// Human-readable representation of FrameworkError.
impl fmt::Display for FrameworkError {

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {

        match self {

            FrameworkError::Framework_InternalError =>
                write!(f, "An internal framework error occurred."),

            FrameworkError::Operation_Timeout =>
                write!(f, "The operation timed out."),

            FrameworkError::Framework_NotInitialized =>
                write!(f, "The framework has not been initialized."),

            FrameworkError::Framework_AlreadyInitialized =>
                write!(f, "The framework is already initialized."),

            FrameworkError::Framework_CannotClose_ModulesStillRunning =>
                write!(f, "The framework shutdown failed: one or more modules are still running."),

            FrameworkError::Module_InitializationFailed =>
                write!(f, "A module failed during its initialization."),

            FrameworkError::Module_NotFound =>
                write!(f, "The requested module was not found."),

            FrameworkError::Module_CloseFailed =>
                write!(f, "The requested module was not found."),

            FrameworkError::Module_ChannelClosed =>
                write!(f, "A module failed during its closing."),

            FrameworkError::Module_AlreadyExists =>
                write!(f, "The requested module already exists."),
            
            FrameworkError::Module_AlreadyInitialized =>
                write!(f, "The requested module is already initialized."),

            FrameworkError::Module_NotInitialized =>
                write!(f, "The requested module has not been initialized."),

            FrameworkError::Module_MessageError(error) =>
                write!(f, "Module message processing failed: {}", error),

            FrameworkError::FatherModule_NotFound =>
                write!(f, "The requested father module was not found."),

        }
    }
}

/// Standard Rust error implementation for FrameworkError.
impl Error for FrameworkError {}