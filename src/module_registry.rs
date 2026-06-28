//! Module registry for MMF.
//!
//! This module stores all registered modules, their hierarchy,
//! their execution state and generates unique ModuleIDs.

use std::collections::HashMap;
use tokio::task::JoinHandle;

use crate::channel::{ModuleNormalSenderChannel, ModulePrioritySenderChannel};
use crate::module::{ModuleTypeID, ModuleID, EntityID, ModuleState, ModuleInfo};
use crate::framework_error::{FrameworkError, MMFPanic};

/// Information associated with a registered module.
#[derive(Debug)]
pub(crate) struct RegistryEntry {

    /// Public identifier of the module type.
    pub(crate) module_type_id: ModuleTypeID,

    /// Identifier of the entity that spawned this module.
    father_id: EntityID,

    /// Current module state.
    pub(crate) module_state: ModuleState,

    /// Channel used to send commands to the module.
    pub(crate) normal_sender_channel: ModuleNormalSenderChannel,

    /// Channel used to send priority commands to the module.
    pub(crate) priority_sender_channel: ModulePrioritySenderChannel,

    /// Tokio task handle associated with the module runtime.
    join_handle: Option<JoinHandle<()>>,

}

impl RegistryEntry {

    /// Create a registry entry for a newly created module.
    fn new(
        module_type_id: ModuleTypeID,
        father_id: EntityID,
        normal_sender_channel: ModuleNormalSenderChannel,
        priority_sender_channel: ModulePrioritySenderChannel,
        join_handle: JoinHandle<()>,
    ) -> Self {

        Self {
            module_type_id,
            father_id,
            module_state: ModuleState::Created,
            normal_sender_channel,
            priority_sender_channel,
            join_handle: Some(join_handle),
        }
    }
}

/// Stores all modules known by the framework.
#[derive(Debug)]
pub(crate) struct Registry {

    /// Maps a ModuleID to its registry entry.
    /// Source of truth for module existence.
    module_map: HashMap<ModuleID, RegistryEntry>,

    /// Maps an entity to its child ModuleIDs.
    child_module_map: HashMap<EntityID, Vec<ModuleID>>,

    /// Stores the instance counter for each module type.
    module_counter_map: HashMap<ModuleTypeID, u64>,
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


    /// Get module registry entry from ModuleID.
    pub(crate) fn get_registry_entry(&self, module_id: &ModuleID) -> Result<&RegistryEntry, FrameworkError> {

        self.module_map.get(module_id).ok_or(FrameworkError::Module_NotFound)
    }


    /// Get mutable module registry entry from ModuleID.
    pub(crate) fn get_registry_entry_mut(&mut self, module_id: &ModuleID) -> Result<&mut RegistryEntry, FrameworkError> {

        self.module_map.get_mut(module_id).ok_or(FrameworkError::Module_NotFound)
    }


    /// Retrieve information about all modules currently
    /// present in the Registry.
    ///
    /// The returned list contains one ModuleInfo entry
    /// per registered module.
    pub(crate) fn get_module_info_list(&self) -> Vec<ModuleInfo> {

        let mut module_info_list = Vec::new();

        for (module_id, registry_entry) in &self.module_map {

            module_info_list.push(
                ModuleInfo {

                    module_id: module_id.clone(),
                    father_id: registry_entry.father_id.clone(),
                    module_state: registry_entry.module_state,
                }
            );
        }

        module_info_list
    }


    /// Get module type id from ModuleID.
    pub(crate) fn get_module_type_id(&self, module_id: &ModuleID) -> Result<ModuleTypeID, FrameworkError> {
        
        Ok(self.get_registry_entry(module_id)?.module_type_id)
    }


    /// Get module channel from ModuleID.
    pub(crate) fn get_module_normal_channel(&self, module_id: &ModuleID) -> Result<ModuleNormalSenderChannel, FrameworkError> {
        
        Ok(self.get_registry_entry(module_id)?.normal_sender_channel.clone())
    }


    /// Get module priority channel from ModuleID.
    pub(crate) fn get_module_priority_channel(&self, module_id: &ModuleID) -> Result<ModulePrioritySenderChannel, FrameworkError> {
        
        Ok(self.get_registry_entry(module_id)?.priority_sender_channel.clone())
    }


    /// Get module state from ModuleID
    pub(crate) fn get_module_state(&self, module_id: &ModuleID) -> Result<ModuleState, FrameworkError> {
        
        Ok(self.get_registry_entry(module_id)?.module_state)
    }


    /// Generate a unique ModuleID for a module type.
    pub(crate) fn generate_module_id(&mut self, module_type_id: ModuleTypeID) -> ModuleID {

        let counter = self.module_counter_map.entry(module_type_id).or_insert(0);
        *counter += 1;

        format!("{}:{:05}", module_type_id, *counter)
    }


    /// Check if the registry is empty.
    pub(crate) fn is_empty(&self) -> bool {

        self.module_map.is_empty()
    }


    /// Register a newly created module.
    pub(crate) fn add_module(&mut self, module_id: &ModuleID, module_type_id: ModuleTypeID, father_id: &EntityID, normal_sender_channel: ModuleNormalSenderChannel, priority_sender_channel: ModulePrioritySenderChannel, join_handle: JoinHandle<()>) -> Result<(), (FrameworkError, JoinHandle<()>)> {
        
        // Prevent duplicate ModuleIDs.
        if self.module_map.contains_key(module_id) {
            return Err((FrameworkError::Module_AlreadyExists, join_handle));
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
                if !Registry::contains_module(&self, father_id_ref) {
                    return Err((FrameworkError::FatherModule_NotFound, join_handle));
                }
            }
            EntityID::External(_) => {}
        }

        // Create the Registry entry associated with the module.
        let registry_entry = RegistryEntry::new(module_type_id, father_id.clone(), normal_sender_channel, priority_sender_channel, join_handle);
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
        let registry_entry = self.get_registry_entry(module_id)?;
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
        
        let registry_entry = self.get_registry_entry_mut(module_id)?;
        registry_entry.module_state = module_state;
        Ok(())
    }


    /// Returns true if the module exists in the Registry.
    pub(crate) fn contains_module(&self, module_id: &ModuleID) -> bool {

        self.module_map.contains_key(module_id)
    }

    
    /// Get the module FatherID from ModuleID.
    pub(crate) fn get_father_id(&self, module_id: &ModuleID) -> Result<EntityID, FrameworkError> {
        
        let registry_entry = self.get_registry_entry(module_id)?;
        Ok(registry_entry.father_id.clone())
    }


    /// Get the child ModuleIDs of an entity.
    pub(crate) fn get_children_ids(&self, entity_id: &EntityID) -> Result<Vec<ModuleID>, FrameworkError> {
        
        // Validate the module if the entity is managed by MMF.
        //
        // External entities are not managed by MMF and therefore
        // cannot be validated.
        if let EntityID::Module(module_id) = entity_id {
            if !Registry::contains_module(&self, module_id) {
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
            if !Registry::contains_module(&self, module_id) {
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
        
        let registry_entry = self.get_registry_entry_mut(module_id)?;
        let join_handle = registry_entry.join_handle.take().expect(MMFPanic::Module_JoinHandleMissing.message());
        Ok(join_handle)
    }


}

