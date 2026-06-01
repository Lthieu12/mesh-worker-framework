//! Framework error definitions.
//!
//! This module contains all errors that can be returned by MMF public APIs.

use std::error::Error;
use std::fmt;

/// Errors returned by MMF functions.
#[derive(Debug)]
pub enum FrameworkError {

    /// The framework has not been initialized.
    FrameworkNotInitialized,

    /// The framework is already initialized.
    FrameworkAlreadyInitialized,

    /// A module failed during its initialization.
    ModuleInitializationFailed,

    /// The requested module was not found.
    ModuleNotFound,

    /// The requested module already exists.
    ModuleAlreadyExists,
}

/// Human-readable representation of FrameworkError.
impl fmt::Display for FrameworkError {

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {

        match self {

            FrameworkError::FrameworkNotInitialized =>
                write!(f, "Framework not initialized"),

            FrameworkError::FrameworkAlreadyInitialized =>
                write!(f, "Framework already initialized"),

            FrameworkError::ModuleInitializationFailed =>
                write!(f, "Module initialization failed"),

            FrameworkError::ModuleNotFound =>
                write!(f, "Module not found"),

            FrameworkError::ModuleAlreadyExists =>
                write!(f, "Module already exists"),
        }
    }
}

/// Standard Rust error implementation for FrameworkError.
impl Error for FrameworkError {}