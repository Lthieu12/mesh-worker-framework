//! Framework error definitions.
//!
//! This file contains all errors that can be returned by MWF public APIs.

use std::error::Error;
use std::fmt;

pub type WorkerError = anyhow::Error;

/// Errors returned by MWF functions.
///
/// Naming convention:
/// - Category_Error
/// - Examples:
///   - Framework_NotInitialized
///   - Worker_NotFound
///   - Worker_ChannelClosed
#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum MWFError {
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
impl fmt::Display for MWFError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MWFError::Operation_Timeout => write!(f, "The operation timed out."),

            MWFError::Framework_NotInitialized => {
                write!(f, "The framework has not been initialized.")
            }

            MWFError::Framework_AlreadyInitialized => {
                write!(f, "The framework is already initialized.")
            }

            MWFError::Framework_CannotClose_workersStillRunning => {
                write!(f, "The framework shutdown failed: one or more workers are still running.")
            }

            MWFError::WorkerType_NotFound => {
                write!(f, "The requested worker type was not found.")
            }

            MWFError::WorkerType_DuplicateWorkerTypeID => {
                write!(f, "WorkerTypeID is already used by another worker type.")
            }

            MWFError::WorkerType_AlreadyRegistered => {
                write!(f, "The requested worker type is already registered.")
            }

            MWFError::WorkerType_NotRegistered => {
                write!(f, "The requested worker type has not been registered.")
            }

            MWFError::Worker_InitializationFailed => {
                write!(f, "A worker failed during its initialization.")
            }

            MWFError::Worker_NotFound => write!(f, "The requested worker was not found."),

            MWFError::Worker_CloseFailed => write!(f, "A worker failed during its closing."),

            MWFError::Worker_ChannelClosed => {
                write!(f, "A worker communication channel is closed.")
            }

            MWFError::Worker_AlreadyExists => {
                write!(f, "The requested worker already exists.")
            }

            MWFError::Worker_AlreadyInitialized => {
                write!(f, "The requested worker is already initialized.")
            }

            MWFError::Worker_NotInitialized => {
                write!(f, "The requested worker has not been initialized.")
            }

            MWFError::Worker_Panicked(panic_message) => {
                write!(f, "A worker has panicked: {}", panic_message)
            }

            MWFError::Worker_MessageError(error) => {
                write!(f, "Worker message processing failed: {}", error)
            }

            MWFError::ParentWorker_NotFound => {
                write!(f, "The requested father worker was not found.")
            }

            MWFError::Message_InstanceIDExhausted => {
                write!(f, "MessageInstanceID counter exhausted.")
            }

            MWFError::Error_InstanceIDExhausted => {
                write!(f, "ErrorInstanceID counter exhausted.")
            }

            MWFError::Logging_InstanceIDExhausted => {
                write!(f, "LoggingInstanceID counter exhausted.")
            }

            MWFError::Logging_LogFileInvalidPath => {
                write!(f, "Invalid log file path.")
            }

            MWFError::Logging_LogFileAccessFailed(error) => {
                write!(f, "Failed to access to log file: {}", error)
            }

            MWFError::BlockingTask_Failed(error) => {
                write!(f, "Blocking task execution failed: {}", error)
            }
        }
    }
}

/// Standard Rust error implementation for FrameworkError.
impl Error for MWFError {}

/// Panic errors of MWF.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub(crate) enum MWFPanic {
    Worker_PriorityChannelClosed,
    Worker_NormalChannelClosed,
    Worker_PendingCommandsEmpty,

    MWFData_ReadLockPoisoned,
    MWFData_WriteLockPoisoned,

    MWFWrkData_ReadLockPoisoned,
    MWFWrkData_WriteLockPoisoned,
    MWFWrkData_Missing,

    Registry_workerMissing,

    worker_JoinHandleMissing,

    MWFErrData_ReadLockPoisoned,
    MWFErrData_WriteLockPoisoned,

    ErrorBuffer_Inconsistency,
    ErrorBuffer_AddErrorFailed,

    MWFLogData_ReadLockPoisoned,
    MWFLogData_WriteLockPoisoned,

    LogBuffer_Inconsistency,
    LogBuffer_AddLogFailed,
}

impl MWFPanic {
    pub(crate) fn message(&self) -> &'static str {
        match self {
            MWFPanic::Worker_PriorityChannelClosed => "Internal MWF error: worker priority command channel unexpectedly closed.",

            MWFPanic::Worker_NormalChannelClosed => "Internal MWF error: worker normal command channel unexpectedly closed.",

            MWFPanic::Worker_PendingCommandsEmpty => "Internal MWF error: worker pending commands cannot be empty.",

            MWFPanic::MWFData_ReadLockPoisoned => "Internal MWF error: MWF_DATA read lock poisoned.",

            MWFPanic::MWFData_WriteLockPoisoned => "Internal MWF error: MWF_DATA write lock poisoned.",

            MWFPanic::MWFWrkData_ReadLockPoisoned => "Internal MWF error: MWFWrk_DATA read lock poisoned.",

            MWFPanic::MWFWrkData_WriteLockPoisoned => "Internal MWF error: MWFWrk_DATA write lock poisoned.",

            MWFPanic::MWFWrkData_Missing => "Internal MWF error: MWFWrk_DATA is unexpectedly missing",

            MWFPanic::Registry_workerMissing => "Internal MWF error: worker unexpectedly missing in registry.",

            MWFPanic::worker_JoinHandleMissing => "Internal MWF error: JoinHandle unexpectedly missing.",

            MWFPanic::MWFErrData_ReadLockPoisoned => "Internal MWF error: MWFErr_DATA read lock poisoned.",

            MWFPanic::MWFErrData_WriteLockPoisoned => "Internal MWF error: MWFErr_DATA write lock poisoned.",

            MWFPanic::ErrorBuffer_Inconsistency => "Internal MWF error: error buffer is inconsistent.",

            MWFPanic::ErrorBuffer_AddErrorFailed => "Internal MWF error: add error to buffer unexpectedly failed.",

            MWFPanic::MWFLogData_ReadLockPoisoned => "Internal MWF error: MWFLog_DATA read lock poisoned.",

            MWFPanic::MWFLogData_WriteLockPoisoned => "Internal MWF error: MWFLog_DATA write lock poisoned.",

            MWFPanic::LogBuffer_Inconsistency => "Internal MWF error: logging buffer is inconsistent.",

            MWFPanic::LogBuffer_AddLogFailed => "Internal MWF error: add log to buffer unexpectedly failed.",
        }
    }
}
