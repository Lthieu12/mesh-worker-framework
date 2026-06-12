//! Module registry for MMF.
//!
//! This module stores all registered modules, their hierarchy,
//! their execution state and generates unique ModuleIDs.

use std::collections::HashMap;
use tokio::task::JoinHandle;

use crate::channel::ModuleSenderChannel;
use crate::module::{Module, ModuleID, EntityID, ModuleState};
use crate::error::FrameworkError;

/// Information associated with a registered module.
#[derive(Debug)]
pub(crate) struct RegistryEntry {

    /// Identifier of the entity that spawned this module.
    father_id: EntityID,

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
        father_id: EntityID,
        module_channel: ModuleSenderChannel,
        join_handle: JoinHandle<()>,
    ) -> Self {

        Self {
            father_id,
            module_state: ModuleState::Created,
            module_channel: module_channel,
            join_handle: Some(join_handle),
        }
    }
}

/// Stores all modules known by the framework.
pub(crate) struct Registry {

    /// Maps a ModuleID to its registry entry.
    /// Source of truth for module existence.
    module_map: HashMap<ModuleID, RegistryEntry>,

    /// Maps an entity to its child ModuleIDs.
    child_module_map: HashMap<EntityID, Vec<ModuleID>>,

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


    /// Get module state from ModuleID
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


    /// Check if the registry is empty.
    pub(crate) fn is_empty(&self) -> bool {

        self.module_map.is_empty()
    }


    /// Register a newly created module.
    pub(crate) fn add_module(&mut self, module_id: &ModuleID, father_id: &EntityID, module_channel: ModuleSenderChannel, join_handle: JoinHandle<()>) -> Result<(), FrameworkError> {
        
        // Prevent duplicate ModuleIDs.
        if self.module_map.contains_key(module_id) {
            return Err(FrameworkError::Module_AlreadyExists);
        }

        // Validate the parent entity.
        //
        // MMF modules must already exist before they can be
        // referenced as a parent.
        //
        // External entities are not managed by MMF and therefore
        // cannot be validated.
        match father_id {
            EntityID::Module(father_id_ref) => {
                if !self.module_map.contains_key(father_id_ref) {
                    return Err(FrameworkError::FatherModule_NotFound);
                }
            }
            EntityID::External(_) => {}
        }

        // Create the Registry entry associated with the module.
        let registry_entry = RegistryEntry::new(father_id.clone(), module_channel, join_handle);
        // Register the module.
        self.module_map.insert(module_id.clone(), registry_entry);

        // Register the parent/child relationship.
        //
        // child_module_map only stores existing relationships.
        //
        // Modules without children do not have an entry in this map.
        self.child_module_map.entry(father_id.clone()).or_insert(Vec::new()).push(module_id.clone());

        Ok(())
    }


    /// Remove a module from the registry.
    pub(crate) fn remove_module(&mut self, module_id: &ModuleID) -> Result<(), FrameworkError> {
        
        // Retrieve the Registry entry associated with the module.
        //
        // The father_id must be retrieved before removing the module
        // from module_map.
        let registry_entry = self.module_map.get(module_id).ok_or(FrameworkError::Module_NotFound)?;
        let father_id = registry_entry.father_id.clone();

        // Remove the module from the Registry.
        self.module_map.remove(module_id);

        // Remove the module from its father's child list.
        //
        // child_module_map only stores existing parent/child
        // relationships.
        if let Some(child_module_list) = self.child_module_map.get_mut(&father_id) {
            // Keep all children except the removed module.
            child_module_list.retain(|child_id| child_id != module_id);

            // Remove the father entry if it no longer has children.
            //
            // child_module_map only stores existing parent/child
            // relationships.
            // An entity with no children does not have an entry in the map.
            if child_module_list.is_empty() {
                self.child_module_map.remove(&father_id);
            }
        }

        // Remove the child list associated with the closed module.
        //
        // A closed module can no longer be the parent of other
        // modules.
        self.child_module_map.remove(&EntityID::Module(module_id.clone()));

        Ok(())
    }


    /// Update the module state of a registered module.
    pub(crate) fn set_module_state(&mut self, module_id: &ModuleID, module_state: ModuleState) -> Result<(), FrameworkError> {
        
        let registry_entry = self.module_map.get_mut(module_id).ok_or(FrameworkError::Module_NotFound)?;
        registry_entry.module_state = module_state;
        Ok(())
    }


    /// Returns true if the module exists in the Registry.
    pub(crate) fn contains_module(&self, module_id: &ModuleID) -> bool {

        self.module_map.contains_key(module_id)
    }

    /// Get the module FatherID from ModuleID.
    pub(crate) fn get_father_id(&self, module_id: &ModuleID) -> Result<EntityID, FrameworkError> {
        
        let registry_entry = self.module_map.get(module_id).ok_or(FrameworkError::Module_NotFound)?;
        Ok(registry_entry.father_id.clone())
    }


    /// Get the child ModuleIDs of an entity.
    pub(crate) fn get_children_ids(&self, entity_id: &EntityID) -> Result<Vec<ModuleID>, FrameworkError> {
        
        // Validate the module if the entity is managed by MMF.
        //
        // External entities are not managed by MMF and therefore
        // cannot be validated.
        if let EntityID::Module(module_id) = entity_id {
            if !self.module_map.contains_key(module_id) {
                return Err(FrameworkError::Module_NotFound);
            }
        }

        // child_module_map only stores existing parent/child
        // relationships.
        //
        // If the entity has no children, return an empty list.
        match self.child_module_map.get(entity_id) {
            Some(child_module_list) => Ok(child_module_list.clone()),
            None => Ok(Vec::new()),
        }
    }


    /// Get the number of child modules of an entity.
    pub(crate) fn get_children_count(&self, entity_id: &EntityID) -> Result<usize, FrameworkError> {
        
        // Validate the module if the entity is managed by MMF.
        //
        // External entities are not managed by MMF and therefore
        // cannot be validated.
        if let EntityID::Module(module_id) = entity_id {
            if !self.module_map.contains_key(module_id) {
                return Err(FrameworkError::Module_NotFound);
            }
        }

        // child_module_map only stores existing parent/child
        // relationships.
        //
        // If the entity has no children, return zero.
        match self.child_module_map.get(entity_id) {
            Some(child_module_list) => Ok(child_module_list.len()),
            None => Ok(0),
        }
    }


    /// Take ownership of the JoinHandle associated with a module.
    pub(crate) fn take_join_handle(&mut self, module_id: &ModuleID) -> Result<JoinHandle<()>, FrameworkError> {
        
        let registry_entry = self.module_map.get_mut(module_id).ok_or(FrameworkError::Module_NotFound)?;
        let join_handle = registry_entry.join_handle.take().ok_or(FrameworkError::Framework_InternalError)?;
        Ok(join_handle)
    }


}

