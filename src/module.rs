//! Module definitions for MMF.
//!
//! This module defines the Module trait and the types used
//! to identify and manage modules within the framework.

use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::message::ModuleMessage;
use crate::string_helper;

/// Function creating a new module instance.
///
/// Each registered module type provides a constructor
/// that MMF stores in the ModuleTypeRegistry.
///
/// Whenever MMF needs to spawn a module from its
/// ModuleTypeID (for example when executing a workflow),
/// this constructor is invoked to create a fresh module
/// instance.
///
/// The returned instance is then owned by the spawned
/// module task.
pub(crate) type ModuleConstructor = fn() -> Box<dyn Module + Send>;

/// Unique identifier of a module type.
///
/// A ModuleTypeID identifies a module *type*, not a module instance.
///
/// Rules:
/// - Must be unique across the whole application.
/// - Should remain stable between software versions if persisted in workflow definitions.
/// - Is case-sensitive.
///
/// Examples:
/// - "Acquisition"
/// - "Filtering"
/// - "FFT"
/// - "SimulationModule"
pub type ModuleTypeID = &'static str;

/// Unique identifier of an external entity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ExternalID(String);

impl ExternalID {
    /// Creates a new external identifier.
    pub fn new(external_id: impl Into<String>) -> Self {
        Self(external_id.into())
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ExternalID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for ExternalID {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for ExternalID {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for ExternalID {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// Unique identifier of a module instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ModuleID(String);

impl ModuleID {
    /// Creates a new external identifier.
    pub fn new(external_id: impl Into<String>) -> Self {
        Self(external_id.into())
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ModuleID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for ModuleID {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for ModuleID {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for ModuleID {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// Generic error type used by on_init(),
/// on_message() and on_close().
pub type ModuleError = anyhow::Error;

/// Identifier of an entity interacting with MMF.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntityID {
    /// MMF module entity.
    Module(ModuleID),
    /// External entity not managed by MMF.
    External(ExternalID),
}

impl std::fmt::Display for EntityID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityID::Module(module_id) => {
                write!(f, "{module_id}")
            }

            EntityID::External(external_id) => {
                write!(f, "External:{external_id}")
            }
        }
    }
}

impl EntityID {
    /// Creates a module entity.
    pub fn new_module(module_id: ModuleID) -> Self {
        Self::Module(module_id)
    }

    /// Creates an external entity.
    pub fn new_external(external_id: impl Into<ExternalID>) -> Self {
        Self::External(external_id.into())
    }
    /// Returns a shortened human-readable representation of the entity.
    ///
    /// Module entities preserve the instance suffix whenever possible.
    /// External entities are truncated at the end.
    pub(crate) fn to_short_string(&self, max_len: usize) -> String {
        match self {
            EntityID::Module(module_id) => {
                let module_id = module_id.as_str();
                if module_id.len() <= max_len {
                    return module_id.to_string();
                }

                // Keep the instance suffix (":00001") visible.
                const SUFFIX_LEN: usize = 6;

                if max_len <= SUFFIX_LEN + 3 {
                    return module_id[..max_len].to_string();
                }

                let prefix_len = max_len - SUFFIX_LEN - 3;

                format!("{}...{}", &module_id[..prefix_len], &module_id[module_id.len() - SUFFIX_LEN..])
            }

            EntityID::External(text) => string_helper::to_short_string(text.as_str(), max_len),
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
    /// Module has panicked and is not usable anymore.
    Panicked,
}

impl ModuleState {
    /// Returns a human-readable representation of the module state.
    fn as_str(&self) -> &'static str {
        match self {
            ModuleState::Created => "Created",
            ModuleState::Initializing => "Initializing",
            ModuleState::InitFailed => "InitFailed",
            ModuleState::Initialized => "Initialized",
            ModuleState::Closing => "Closing",
            ModuleState::CloseFailed => "CloseFailed",
            ModuleState::Closed => "Closed",
            ModuleState::Panicked => "Panicked",
        }
    }
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
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub module_id: ModuleID,
    pub father_id: EntityID,
    pub module_state: ModuleState,
}

impl ModuleInfo {
    /// Converts the module information into a human-readable terminal string.
    fn to_terminal_string(&self) -> String {
        format!(
            "{:width_module$} | {:width_father$} | {:width_state$}",
            EntityID::Module(self.module_id.clone()).to_short_string(terminal_module_column::MODULE_ID_LEN,),
            self.father_id.to_short_string(terminal_module_column::FATHER_ID_LEN,),
            self.module_state.as_str(),
            width_module = terminal_module_column::MODULE_ID_LEN,
            width_father = terminal_module_column::FATHER_ID_LEN,
            width_state = terminal_module_column::STATE_LEN,
        )
    }

    /// Converts the module information into a CSV row.
    fn to_csv_string(&self) -> String {
        format!("{},{},{}", string_helper::csv_escape(self.module_id.as_str()), string_helper::csv_escape(&self.father_id.to_string()), string_helper::csv_escape(self.module_state.as_str()),)
    }
}

/// Output format used to represent module information.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ModuleOutputFormat {
    Terminal,
    CSV,
}

/// Terminal module column widths.
mod terminal_module_column {

    pub(crate) const MODULE_ID_LEN: usize = 30;
    pub(crate) const FATHER_ID_LEN: usize = 30;
    pub(crate) const STATE_LEN: usize = 14;
}

/// List of module information returned by MMFMod.
#[derive(Debug, Clone)]
pub struct ModuleList {
    pub module_info_list: Vec<ModuleInfo>,
}

impl std::fmt::Display for ModuleList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        writeln!(f, "{}", Self::get_header(ModuleOutputFormat::Terminal))?;

        for line in self.to_string_list(ModuleOutputFormat::Terminal) {
            writeln!(f, "{line}")?;
        }
        Ok(())
    }
}

impl ModuleList {
    /// Returns the column header corresponding to the requested format.
    pub fn get_header(format: ModuleOutputFormat) -> String {
        match format {
            ModuleOutputFormat::Terminal => {
                let header = format!("{:<width_module$} | {:<width_father$} | {:<width_state$}", "ModuleID", "FatherID", "State", width_module = terminal_module_column::MODULE_ID_LEN, width_father = terminal_module_column::FATHER_ID_LEN, width_state = terminal_module_column::STATE_LEN,);

                let separator = "-".repeat(header.len());

                format!("{header}\n{separator}")
            }

            ModuleOutputFormat::CSV => String::from("ModuleID,FatherID,State"),
        }
    }

    /// Converts a module information list into strings.
    pub fn to_string_list(&self, format: ModuleOutputFormat) -> Vec<String> {
        let mut module_info_list = self.module_info_list.clone();
        module_info_list.sort_by(|a, b| a.module_id.cmp(&b.module_id));

        match format {
            ModuleOutputFormat::Terminal => module_info_list.iter().map(|module_info| module_info.to_terminal_string()).collect(),

            ModuleOutputFormat::CSV => module_info_list.iter().map(|module_info| module_info.to_csv_string()).collect(),
        }
    }
}

/// Interface implemented by all MMF modules.
///
/// MMF creates module instances dynamically by calling
/// `Module::new()` on the registered module type.
///
/// MMF stores module instances behind `Box<dyn Module>`.
///
/// The `Self: Sized` bounds on `module_type_id()` and `new()` are
/// required to keep this trait object-safe. Without these bounds,
/// `Module` could no longer be used as `dyn Module`.
#[async_trait]
pub trait Module {
    /// Returns the unique identifier of this module type.
    ///
    /// Every module implementation must provide a unique ModuleTypeID.
    ///
    /// This associated function is only available on concrete module
    /// types (`Self: Sized`). It is intentionally excluded from trait
    /// objects (`dyn Module`) so that MMF can store modules behind
    /// `Box<dyn Module>`.
    fn module_type_id() -> ModuleTypeID
    where
        Self: Sized;

    /// Creates a new module instance.
    ///
    /// MMF calls this function whenever a new module
    /// instance must be spawned from its registered
    /// ModuleTypeID.
    ///
    /// The returned instance must be in its initial state.
    /// Runtime resources should be acquired later during
    /// on_init().
    ///
    /// This associated function is only available on concrete module
    /// types (`Self: Sized`). It is intentionally excluded from trait
    /// objects (`dyn Module`) so that MMF can store modules behind
    /// `Box<dyn Module>`.
    fn new() -> Self
    where
        Self: Sized;

    /// Called once when the module is initialized.
    async fn on_init(&mut self, module_id: ModuleID, father_id: EntityID, lifecycle_data: Bytes) -> Result<(), ModuleError>;

    /// Called when a message is received by the module.
    async fn on_message(&mut self, message: ModuleMessage) -> Result<(), ModuleError>;

    /// Called once before the module is closed.
    async fn on_close(&mut self, lifecycle_data: Bytes) -> Result<(), ModuleError>;
}
