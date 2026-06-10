//! Logging configuration for MMF.
//!
//! This module contains the structures used to configure
//! framework logging behavior.

use crate::module::ModuleID;

/// Defines how logging is performed by the framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogMode {

    /// Disable all logging.
    NoLog,

    /// Log only selected modules.
    LogSelected,

    /// Log all modules.
    LogAll,
}

/// Stores the current logging configuration.
pub(crate) struct LoggingData {

    /// Current logging mode.
    pub log_mode: LogMode,

    /// List of modules to log when LogSelected mode is active.
    pub logged_module_list: Vec<ModuleID>,
}

impl LoggingData {

    /// Create a new logging configuration with logging disabled.
    pub(crate) fn new() -> Self {
        Self {
            log_mode: LogMode::NoLog,
            logged_module_list: Vec::new(),
        }
    }
    
}