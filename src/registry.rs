//! Module registry for MMF.
//!
//! This module stores all registered modules, their hierarchy,
//! their execution state and generates unique ModuleIDs.

use std::collections::HashMap;

use crate::module::{Module, ModuleID, ModuleState};
use crate::error::FrameworkError;

/// Information associated with a registered module.
#[derive(Debug, Clone)]
pub(crate) struct RegistryEntry {

    /// Parent module identifier. None indicates a root module.
    father_module_id: Option<ModuleID>,

    /// Current module state.
    module_state: ModuleState,

}

impl RegistryEntry {

    /// Create a registry entry for a newly created module.
    fn new(father_module_id: Option<ModuleID>) -> Self {
        Self {
            father_module_id,
            module_state: ModuleState::Created,
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

    /// Generate a unique ModuleID for a module type.
    pub(crate) fn generate_module_id<T>(&mut self) -> ModuleID where T: Module {
        let module_name = std::any::type_name::<T>().to_string();
        let counter = self.module_counter_map.entry(module_name.clone()).or_insert(0);
        *counter += 1;
        format!("{}:{:05}", module_name, *counter)
    }

    /// Register a newly created module.
    pub(crate) fn add_module(&mut self, module_id: ModuleID, father_module_id: Option<ModuleID>) -> Result<(), FrameworkError> {

        if self.module_map.contains_key(&module_id) {
            return Err(FrameworkError::ModuleAlreadyExists);
        }

        let registry_entry = RegistryEntry::new(father_module_id.clone());

        self.module_map.insert(module_id.clone(), registry_entry);

        if let Some(father_module_id) = father_module_id {
            self.child_module_map.entry(father_module_id).or_insert(Vec::new()).push(module_id);
        }

        Ok(())
    }

    /// Remove a module from the registry.
    pub(crate) fn remove_module(&mut self, module_id: &ModuleID) -> Result<(), FrameworkError> {

        let registry_entry = self.module_map.get(module_id).ok_or(FrameworkError::ModuleNotFound)?;
        let father_module_id = registry_entry.father_module_id.clone();

        self.module_map.remove(module_id);

        if let Some(father_module_id) = father_module_id {

            if let Some(child_module_list) = self.child_module_map.get_mut(&father_module_id) {

                child_module_list.retain(|child_id| child_id != module_id);

                if child_module_list.is_empty() {
                    self.child_module_map.remove(&father_module_id);
                }
            }
        }

        self.child_module_map.remove(module_id);

        Ok(())
    }

}


#[test]
fn test_add_remove_module() {

    let mut registry = Registry::new();

    let module_a = "ModuleA:00001".to_string();
    let module_b = "ModuleB:00001".to_string();
    let module_c = "ModuleC:00001".to_string();

    assert!(registry.add_module(module_a.clone(), None).is_ok());
    assert!(registry.add_module(module_b.clone(), Some(module_a.clone())).is_ok());
    assert!(registry.add_module(module_c.clone(), Some(module_a.clone())).is_ok());

    assert_eq!(registry.module_map.len(), 3);

    assert_eq!(
        registry.child_module_map.get(&module_a),
        Some(&vec![module_b.clone(), module_c.clone()])
    );

    assert!(registry.remove_module(&module_b).is_ok());

    assert_eq!(registry.module_map.len(), 2);

    assert_eq!(
        registry.child_module_map.get(&module_a),
        Some(&vec![module_c.clone()])
    );

    assert!(registry.remove_module(&module_a).is_ok());

    assert_eq!(registry.module_map.len(), 1);

    assert!(registry.module_map.contains_key(&module_c));

    assert!(!registry.child_module_map.contains_key(&module_a));
}

#[test]
fn test_add_existing_module() {

    let mut registry = Registry::new();

    let module_a = "ModuleA:00001".to_string();

    assert!(registry.add_module(module_a.clone(), None).is_ok());

    assert!(
        matches!(
            registry.add_module(module_a.clone(), None),
            Err(FrameworkError::ModuleAlreadyExists)
        )
    );
}

#[test]
fn test_remove_unknown_module() {

    let mut registry = Registry::new();

    assert!(
        matches!(
            registry.remove_module(&"Unknown".to_string()),
            Err(FrameworkError::ModuleNotFound)
        )
    );
}