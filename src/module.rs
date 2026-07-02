//! Module definitions for MMF.
//!
//! This module defines the Module trait and the types used
//! to identify and manage modules within the framework.

use async_trait::async_trait;
use bytes::Bytes;

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

                if module_id.len() <= max_len {
                    return module_id.clone();
                }

                // Keep the instance suffix (":00001") visible.
                const SUFFIX_LEN: usize = 6;

                if max_len <= SUFFIX_LEN + 3 {
                    return module_id[..max_len].to_string();
                }

                let prefix_len = max_len - SUFFIX_LEN - 3;

                format!("{}...{}", &module_id[..prefix_len], &module_id[module_id.len() - SUFFIX_LEN..])
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
    /// Module has panicked and is not usable anymore.
    Panicked,
}

impl ModuleState {

    /// Returns a human-readable representation of the module state.
    fn to_string(&self) -> &'static str {

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
            EntityID::Module(self.module_id.clone()).to_short_string(
                terminal_module_column::MODULE_ID_LEN,
            ),
            self.father_id.to_short_string(
                terminal_module_column::FATHER_ID_LEN,
            ),
            self.module_state.to_string(),
            width_module = terminal_module_column::MODULE_ID_LEN,
            width_father = terminal_module_column::FATHER_ID_LEN,
            width_state = terminal_module_column::STATE_LEN,
        )
    }

    /// Converts the module information into a CSV row.
    fn to_csv_string(&self) -> String {

        format!(
            "{},{},{}",
            string_helper::csv_escape(&self.module_id),
            string_helper::csv_escape(&self.father_id.to_string()),
            string_helper::csv_escape(self.module_state.to_string()),
        )
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
    pub(crate) const STATE_LEN: usize = 12;
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

                let header = format!(
                    "{:<width_module$} | {:<width_father$} | {:<width_state$}",
                    "ModuleID",
                    "FatherID",
                    "State",
                    width_module = terminal_module_column::MODULE_ID_LEN,
                    width_father = terminal_module_column::FATHER_ID_LEN,
                    width_state = terminal_module_column::STATE_LEN,
                );

                let separator = "-".repeat(header.len());

                format!("{header}\n{separator}")
            }

            ModuleOutputFormat::CSV => {

                String::from("ModuleID,FatherID,State")
            }
        }
    }

    /// Converts a module information list into strings.
    pub fn to_string_list(&self, format: ModuleOutputFormat) -> Vec<String> {

        let mut module_info_list = self.module_info_list.clone();
        module_info_list.sort_by(|a, b| a.module_id.cmp(&b.module_id));

        match format {

            ModuleOutputFormat::Terminal => {

                module_info_list.iter().map(|module_info| module_info.to_terminal_string()).collect()
            }

            ModuleOutputFormat::CSV => {

                module_info_list.iter().map(|module_info| module_info.to_csv_string()).collect()
            }
        }
    }
}


/// Interface implemented by all MMF modules.
///
/// MMF creates module instances dynamically by calling
/// `Module::new()` on the registered module type.
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
    where Self: Sized;

    /// Creates a new module instance.
    ///
    /// MMF calls this function whenever a new module
    /// instance must be spawned from its registered
    /// ModuleTypeID.
    ///
    /// The returned instance must be in its initial state.
    /// Runtime resources should be acquired later during
    /// on_init().
    fn new() -> Self
    where Self: Sized;

    /// Called once when the module is initialized.
    async fn on_init(&mut self, module_id: ModuleID, father_id: EntityID, lifecycle_data: Bytes) -> Result<(), ModuleError>;

    /// Called when a message is received by the module.
    async fn on_message(&mut self, message: ModuleMessage) -> Result<(), ModuleError>;

    /// Called once before the module is closed.
    async fn on_close(&mut self, lifecycle_data: Bytes) -> Result<(), ModuleError>;
}