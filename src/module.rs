//! Module definitions for MMF.
//!
//! This module defines the Module trait and the types used
//! to identify and manage modules within the framework.

use async_trait::async_trait;

use crate::message::FrameworkMessage;
use crate::string_helper;

/// Unique identifier of a module instance.
pub type ModuleID = String;

/// Generic error type used by on_init(),
/// on_message() and on_close().
pub type ModuleError = anyhow::Error;

/// Identifier of an entity interacting with MMF.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntityID {
    /// MMF module entity.
    Module(ModuleID),
    /// External entity not managed by MMF.
    External(String),
}

impl EntityID {

    /// Returns a human-readable representation of the entity.
    pub(crate) fn to_string(&self) -> String {

        match self {

            EntityID::Module(module_id) => module_id.to_string(),
            EntityID::External(name) => name.clone(),
        }
    }


    /// Returns a shortened human-readable representation of the entity.
    ///
    /// Module entities preserve the instance suffix whenever possible.
    /// External entities are truncated at the end.
    pub(crate) fn to_short_string(&self, max_len: usize) -> String {

        match self {

            EntityID::Module(module_id) => {

                let text = module_id.to_string();
                if text.len() <= max_len {
                    return text;
                }

                // Keep the instance suffix (":000001") visible.
                const SUFFIX_LEN: usize = 7;
                if max_len <= SUFFIX_LEN + 3 {
                    return text[..max_len].to_string();
                }
                let prefix_len = max_len - SUFFIX_LEN - 3;
                format!("{}...{}", &text[..prefix_len], &text[text.len() - SUFFIX_LEN..])
            }

            EntityID::External(text) => {

                string_helper::to_short_string(text, max_len)
            }
        }
    }
}


/// Current state of a module within the framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleState {
    /// Module has been created but not initialized.
    Created,
    /// Module initialization is in progress.
    Initializing,
    /// Module initialization has failed.
    InitFailed,
    /// Module initialization has completed successfully.
    Initialized,
    /// Module Closing is in progress.
    Closing,
    /// Module close has failed.
    CloseFailed,
    /// Module has been closed and is not usable anymore.
    Closed,
}


/// Current status of a module within the framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleStatus {

    pub module_state: ModuleState,

    pub normal_channel_size: usize,
    pub normal_channel_capacity: usize,

    pub priority_channel_size: usize,
    pub priority_channel_capacity: usize,
}


/// Information describing a currently registered module.
///
/// This structure provides a snapshot of the module
/// state at the time it was retrieved.
///
/// It is primarily intended for:
/// - Debugging.
/// - Monitoring.
/// - User interfaces.
/// - Logging configuration tools.
pub struct ModuleInfo {

    pub module_id: ModuleID,
    pub father_id: EntityID,
    pub module_state: ModuleState,
}


/// Interface that must be implemented by all MMF modules.
#[async_trait]
pub trait Module {

    /// Called once when the module is initialized.
    async fn on_init(&mut self, module_id: &ModuleID, father_id: &EntityID) -> Result<(), ModuleError>;

    /// Called when a message is received by the module.
    async fn on_message(&mut self, message: FrameworkMessage) -> Result<(), ModuleError>;

    /// Called once before the module is closed.
    async fn on_close(&mut self) -> Result<(), ModuleError>;
}