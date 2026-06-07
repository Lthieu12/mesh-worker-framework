//! Module registry for MMF.
//!
//! This module stores all registered modules, their hierarchy,
//! their execution state and generates unique ModuleIDs.

use std::collections::HashMap;
use tokio::task::JoinHandle;

use crate::channel::ModuleSenderChannel;
use crate::module::{Module, ModuleID, ModuleState};
use crate::error::FrameworkError;

/// Information associated with a registered module.
#[derive(Debug)]
pub(crate) struct RegistryEntry {

    /// Parent module identifier. None indicates a root module.
    father_module_id: Option<ModuleID>,

    /// Current module state.
    module_state: ModuleState,

    /// Channel used to send commands to the module.
    module_channel: ModuleSenderChannel,

    /// Tokio task handle associated with the module runtime.
    join_handle: Option<JoinHandle<()>>,

}

impl RegistryEntry {

    /// Create a registry entry for a newly created module.
    fn new(
        father_module_id: Option<ModuleID>,
        module_channel: ModuleSenderChannel,
        join_handle: JoinHandle<()>,
    ) -> Self {

        Self {
            father_module_id,
            module_state: ModuleState::Created,
            module_channel: module_channel,
            join_handle: Some(join_handle),
        }
    }
}

/// Stores all modules known by the framework.
pub(crate) struct Registry {

    /// Maps a ModuleID to its registry entry.
    module_map: HashMap<ModuleID, RegistryEntry>,

    /// Maps a parent ModuleID to its child ModuleIDs.
    child_module_map: HashMap<ModuleID, Vec<ModuleID>>,

    /// Stores the instance counter for each module type.
    module_counter_map: HashMap<String, u64>,
}

impl Registry {

    /// Create an empty registry.
    pub(crate) fn new() -> Self {
        Self {
            module_map: HashMap::new(),
            child_module_map: HashMap::new(),
            module_counter_map: HashMap::new(),
        }
    }

    /// Get module channel from ModuleID.
    pub(crate) fn get_module_channel(&self, module_id: &ModuleID) -> Result<ModuleSenderChannel, FrameworkError> {

        let registry_entry = self.module_map.get(module_id).ok_or(FrameworkError::Module_NotFound)?;

        Ok(registry_entry.module_channel.clone())
    }

    pub(crate) fn get_module_state(&self, module_id: &ModuleID) -> Result<ModuleState, FrameworkError> {

        let registry_entry = self.module_map.get(module_id).ok_or(FrameworkError::Module_NotFound)?;

        Ok(registry_entry.module_state)
    }

    /// Generate a unique ModuleID for a module type.
    pub(crate) fn generate_module_id<T>(&mut self) -> ModuleID where T: Module {
        let module_name = std::any::type_name::<T>().to_string();
        let counter = self.module_counter_map.entry(module_name.clone()).or_insert(0);
        *counter += 1;
        format!("{}:{:05}", module_name, *counter)
    }

    /// Register a newly created module.
    pub(crate) fn add_module(&mut self, module_id: ModuleID, father_module_id: Option<ModuleID>, module_channel: ModuleSenderChannel, join_handle: JoinHandle<()>,) -> Result<(), FrameworkError> {

        if self.module_map.contains_key(&module_id) {
            return Err(FrameworkError::Module_AlreadyExists);
        }

        if let Some(father_module_id_ref) = &father_module_id {

            if !self.module_map.contains_key(father_module_id_ref) {
                return Err(FrameworkError::Module_NotFound);
            }
        }

        let registry_entry = RegistryEntry::new(father_module_id.clone(), module_channel, join_handle);

        self.module_map.insert(module_id.clone(), registry_entry);

        if let Some(father_module_id_ref) = father_module_id {
            self.child_module_map.entry(father_module_id_ref).or_insert(Vec::new()).push(module_id);
        }

        Ok(())
    }

    /// Remove a module from the registry.
    pub(crate) fn remove_module(&mut self, module_id: &ModuleID) -> Result<(), FrameworkError> {

        let registry_entry = self.module_map.get(module_id).ok_or(FrameworkError::Module_NotFound)?;
        let father_module_id = registry_entry.father_module_id.clone();

        self.module_map.remove(module_id);

        if let Some(father_module_id_ref) = father_module_id {

            if let Some(child_module_list) = self.child_module_map.get_mut(&father_module_id_ref) {

                child_module_list.retain(|child_id| child_id != module_id);

                if child_module_list.is_empty() {
                    self.child_module_map.remove(&father_module_id_ref);
                }
            }
        }

        self.child_module_map.remove(module_id);

        Ok(())
    }

    /// Update the state of a registered module.
    pub(crate) fn set_module_state(&mut self, module_id: &ModuleID, module_state: ModuleState) -> Result<(), FrameworkError> {

        let registry_entry = self.module_map.get_mut(module_id).ok_or(FrameworkError::Module_NotFound)?;

        registry_entry.module_state = module_state;

        Ok(())
    }

    /// Take ownership of the JoinHandle associated with a module.
    pub(crate) fn take_join_handle(&mut self, module_id: &ModuleID) -> Result<JoinHandle<()>, FrameworkError> {

        let registry_entry = self.module_map.get_mut(module_id).ok_or(FrameworkError::Module_NotFound)?;

        let join_handle = registry_entry.join_handle.take().ok_or(FrameworkError::Framework_InternalError)?;

        Ok(join_handle)
    }
    
}

