//! Framework error definitions.
//!
//! This module contains all errors that can be returned by MMF public APIs.

use std::error::Error;
use std::fmt;


pub type ModuleError = anyhow::Error;


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
    Module_MessageError(anyhow::Error),

    /// The requested father module was not found.
    FatherModule_NotFound,

    /// The MessageInstanceID counter has reached its maximum value
    /// and can no longer generate unique identifiers.
    Message_InstanceIDExhausted,

    /// The ErrorInstanceID counter has reached its maximum value
    /// and can no longer generate unique identifiers.
    Error_InstanceIDExhausted,
}

/// Human-readable representation of FrameworkError.
impl fmt::Display for FrameworkError {

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {

        match self {

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
                write!(f, "A module failed during its closing."),

            FrameworkError::Module_ChannelClosed =>
                write!(f, "A module communication channel is closed."),

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

            FrameworkError::Message_InstanceIDExhausted =>
                write!(f, "MessageInstanceID counter exhausted."),

            FrameworkError::Error_InstanceIDExhausted =>
                write!(f, "ErrorInstanceID counter exhausted."),

        }
    }
}

/// Standard Rust error implementation for FrameworkError.
impl Error for FrameworkError {}


/// Panic errors of MMF. 
#[derive(Debug)]
#[allow(non_camel_case_types)]
pub(crate) enum MMFPanic {

    Module_PriorityChannelClosed,
    Module_NormalChannelClosed,

    MMFData_ReadLockPoisoned,
    MMFData_WriteLockPoisoned,
    MMFData_Missing,

    Registry_ModuleMissing,

    Module_JoinHandleMissing,

    MMFErrData_ReadLockPoisoned,
    MMFErrData_WriteLockPoisoned,

    ErrorBuffer_Inconsistency,
    ErrorBuffer_AddErrorFailed,
}

impl MMFPanic {

    pub(crate) fn message(&self) -> &'static str {

        match self {

            MMFPanic::Module_PriorityChannelClosed => {
                "Internal MMF error: module priority command channel unexpectedly closed."
            }

            MMFPanic::Module_NormalChannelClosed => {
                "Internal MMF error: module normal command channel unexpectedly closed."
            }

            MMFPanic::MMFData_ReadLockPoisoned => {
                "Internal MMF error: MMF_DATA read lock poisoned."
            }

            MMFPanic::MMFData_WriteLockPoisoned => {
                "Internal MMF error: MMF_DATA write lock poisoned."
            }

            MMFPanic::MMFData_Missing => {
                "Internal MMF error: MMF_DATA is unexpectedly missing"
            }

            MMFPanic::Registry_ModuleMissing => {
                "Internal MMF error: module unexpectedly missing in registry."
            }

            MMFPanic::Module_JoinHandleMissing => {
                "Internal MMF error: JoinHandle unexpectedly missing."
            }

            MMFPanic::MMFErrData_ReadLockPoisoned => {
                "Internal MMF error: MMFErr_DATA read lock poisoned."
            }

            MMFPanic::MMFErrData_WriteLockPoisoned => {
                "Internal MMF error: MMFErr_DATA write lock poisoned."
            }

            MMFPanic::ErrorBuffer_Inconsistency => {
                "Internal MMF error: error buffer is inconsistent."
            }

            MMFPanic::ErrorBuffer_AddErrorFailed => {
                "Internal MMF error: add error to buffer unexpectedly failed."
            }
        }
    }
}