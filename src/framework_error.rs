//! Framework error definitions.
//!
//! This file contains all errors that can be returned by MMF public APIs.

use std::error::Error;
use std::fmt;

pub type WorkerError = anyhow::Error;

/// Errors returned by MMF functions.
///
/// Naming convention:
/// - Category_Error
/// - Examples:
///   - Framework_NotInitialized
///   - worker_NotFound
///   - worker_ChannelClosed
#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum MMFError {
    /// The operation timed out.
    Operation_Timeout,

    /// The framework has not been initialized.
    Framework_NotInitialized,

    /// The framework is already initialized.
    Framework_AlreadyInitialized,

    /// The framework shutdown failed: one or more workers are still running.
    Framework_CannotClose_workersStillRunning,

    /// The requested worker type was not found.
    /// Internal registry lookup error.
    WorkerType_NotFound,

    /// WorkerTypeID is already used by another worker type.
    WorkerType_DuplicateWorkerTypeID,

    /// The requested worker type is already registered.
    WorkerType_AlreadyRegistered,

    /// The requested worker type has not been registered.
    /// Public API error.
    WorkerType_NotRegistered,

    /// A worker failed during its initialization.
    Worker_InitializationFailed,

    /// The requested worker was not found.
    Worker_NotFound,

    /// A worker failed during its closing.
    Worker_CloseFailed,

    /// A worker communication channel is closed.
    Worker_ChannelClosed,

    /// The requested worker already exists.
    Worker_AlreadyExists,

    /// The requested worker is already initialized.
    Worker_AlreadyInitialized,

    /// The requested worker has not been initialized.
    Worker_NotInitialized,

    /// A worker has panicked.
    Worker_Panicked(anyhow::Error),

    /// Error returned by a worker while processing
    /// a message sent through send_message_and_wait_reply().
    Worker_MessageError(anyhow::Error),

    /// The requested father worker was not found.
    ParentWorker_NotFound,

    /// The MessageInstanceID counter has reached its maximum value
    /// and can no longer generate unique identifiers.
    Message_InstanceIDExhausted,

    /// The ErrorInstanceID counter has reached its maximum value
    /// and can no longer generate unique identifiers.
    Error_InstanceIDExhausted,

    /// The LoggingInstanceID counter has reached its maximum value
    /// and can no longer generate unique identifiers.
    Logging_InstanceIDExhausted,

    /// Invalid log file path.
    Logging_LogFileInvalidPath,

    /// Failed to access to log file.
    Logging_LogFileAccessFailed(anyhow::Error),

    /// Blocking task execution failed.
    BlockingTask_Failed(anyhow::Error),
}

/// Human-readable representation of FrameworkError.
impl fmt::Display for MMFError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MMFError::Operation_Timeout => write!(f, "The operation timed out."),

            MMFError::Framework_NotInitialized => {
                write!(f, "The framework has not been initialized.")
            }

            MMFError::Framework_AlreadyInitialized => {
                write!(f, "The framework is already initialized.")
            }

            MMFError::Framework_CannotClose_workersStillRunning => {
                write!(f, "The framework shutdown failed: one or more workers are still running.")
            }

            MMFError::WorkerType_NotFound => {
                write!(f, "The requested worker type was not found.")
            }

            MMFError::WorkerType_DuplicateWorkerTypeID => {
                write!(f, "WorkerTypeID is already used by another worker type.")
            }

            MMFError::WorkerType_AlreadyRegistered => {
                write!(f, "The requested worker type is already registered.")
            }

            MMFError::WorkerType_NotRegistered => {
                write!(f, "The requested worker type has not been registered.")
            }

            MMFError::Worker_InitializationFailed => {
                write!(f, "A worker failed during its initialization.")
            }

            MMFError::Worker_NotFound => write!(f, "The requested worker was not found."),

            MMFError::Worker_CloseFailed => write!(f, "A worker failed during its closing."),

            MMFError::Worker_ChannelClosed => {
                write!(f, "A worker communication channel is closed.")
            }

            MMFError::Worker_AlreadyExists => {
                write!(f, "The requested worker already exists.")
            }

            MMFError::Worker_AlreadyInitialized => {
                write!(f, "The requested worker is already initialized.")
            }

            MMFError::Worker_NotInitialized => {
                write!(f, "The requested worker has not been initialized.")
            }

            MMFError::Worker_Panicked(panic_message) => {
                write!(f, "A worker has panicked: {}", panic_message)
            }

            MMFError::Worker_MessageError(error) => {
                write!(f, "Worker message processing failed: {}", error)
            }

            MMFError::ParentWorker_NotFound => {
                write!(f, "The requested father worker was not found.")
            }

            MMFError::Message_InstanceIDExhausted => {
                write!(f, "MessageInstanceID counter exhausted.")
            }

            MMFError::Error_InstanceIDExhausted => {
                write!(f, "ErrorInstanceID counter exhausted.")
            }

            MMFError::Logging_InstanceIDExhausted => {
                write!(f, "LoggingInstanceID counter exhausted.")
            }

            MMFError::Logging_LogFileInvalidPath => {
                write!(f, "Invalid log file path.")
            }

            MMFError::Logging_LogFileAccessFailed(error) => {
                write!(f, "Failed to access to log file: {}", error)
            }

            MMFError::BlockingTask_Failed(error) => {
                write!(f, "Blocking task execution failed: {}", error)
            }
        }
    }
}

/// Standard Rust error implementation for FrameworkError.
impl Error for MMFError {}

/// Panic errors of MMF.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub(crate) enum MMFPanic {
    Worker_PriorityChannelClosed,
    Worker_NormalChannelClosed,
    Worker_PendingCommandsEmpty,

    MMFData_ReadLockPoisoned,
    MMFData_WriteLockPoisoned,

    MMFWrkData_ReadLockPoisoned,
    MMFWrkData_WriteLockPoisoned,
    MMFWrkData_Missing,

    Registry_workerMissing,

    worker_JoinHandleMissing,

    MMFErrData_ReadLockPoisoned,
    MMFErrData_WriteLockPoisoned,

    ErrorBuffer_Inconsistency,
    ErrorBuffer_AddErrorFailed,

    MMFLogData_ReadLockPoisoned,
    MMFLogData_WriteLockPoisoned,

    LogBuffer_Inconsistency,
    LogBuffer_AddLogFailed,
}

impl MMFPanic {
    pub(crate) fn message(&self) -> &'static str {
        match self {
            MMFPanic::Worker_PriorityChannelClosed => "Internal MMF error: worker priority command channel unexpectedly closed.",

            MMFPanic::Worker_NormalChannelClosed => "Internal MMF error: worker normal command channel unexpectedly closed.",

            MMFPanic::Worker_PendingCommandsEmpty => "Internal MMF error: worker pending commands cannot be empty.",

            MMFPanic::MMFData_ReadLockPoisoned => "Internal MMF error: MMF_DATA read lock poisoned.",

            MMFPanic::MMFData_WriteLockPoisoned => "Internal MMF error: MMF_DATA write lock poisoned.",

            MMFPanic::MMFWrkData_ReadLockPoisoned => "Internal MMF error: MMFWrk_DATA read lock poisoned.",

            MMFPanic::MMFWrkData_WriteLockPoisoned => "Internal MMF error: MMFWrk_DATA write lock poisoned.",

            MMFPanic::MMFWrkData_Missing => "Internal MMF error: MMFWrk_DATA is unexpectedly missing",

            MMFPanic::Registry_workerMissing => "Internal MMF error: worker unexpectedly missing in registry.",

            MMFPanic::worker_JoinHandleMissing => "Internal MMF error: JoinHandle unexpectedly missing.",

            MMFPanic::MMFErrData_ReadLockPoisoned => "Internal MMF error: MMFErr_DATA read lock poisoned.",

            MMFPanic::MMFErrData_WriteLockPoisoned => "Internal MMF error: MMFErr_DATA write lock poisoned.",

            MMFPanic::ErrorBuffer_Inconsistency => "Internal MMF error: error buffer is inconsistent.",

            MMFPanic::ErrorBuffer_AddErrorFailed => "Internal MMF error: add error to buffer unexpectedly failed.",

            MMFPanic::MMFLogData_ReadLockPoisoned => "Internal MMF error: MMFLog_DATA read lock poisoned.",

            MMFPanic::MMFLogData_WriteLockPoisoned => "Internal MMF error: MMFLog_DATA write lock poisoned.",

            MMFPanic::LogBuffer_Inconsistency => "Internal MMF error: logging buffer is inconsistent.",

            MMFPanic::LogBuffer_AddLogFailed => "Internal MMF error: add log to buffer unexpectedly failed.",
        }
    }
}
