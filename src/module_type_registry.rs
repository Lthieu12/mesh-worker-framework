//! Module type registry for MMF.
//!
//! This module stores all module types known by MMF.
//!
//! Registered module types are identified by a ModuleTypeID
//! and associated with their Rust module type.

use std::collections::HashMap;

use crate::module::ModuleTypeID;
use crate::framework_error::{FrameworkError};

/// Information associated with a registered module type.
#[derive(Debug)]
pub(crate) struct ModuleTypeEntry {

    /// Fully qualified Rust type name.
    pub(crate) module_rust_type_name: &'static str,
}

/// Stores all module types known by the framework.
#[derive(Debug)]
pub(crate) struct ModuleTypeRegistry {

    /// Maps a ModuleTypeID to its associated Rust module type.
    module_type_map: HashMap<ModuleTypeID, ModuleTypeEntry>,
}

impl ModuleTypeRegistry {

    /// Create an empty module type registry.
    pub(crate) fn new() -> Self {

        Self {
            module_type_map: HashMap::new(),
        }
    }


    /// Register a module type.
    pub(crate) fn add_module_type(&mut self, module_type_id: ModuleTypeID, module_rust_type_name: &'static str) {

        self.module_type_map.insert(module_type_id, ModuleTypeEntry {
                module_rust_type_name,
            },
        );
    }


    /// Get the Rust type name associated with a ModuleTypeID.
    pub(crate) fn get_module_rust_type_name(&self, module_type_id: ModuleTypeID) -> Result<&'static str, FrameworkError> {

            let module_type_entry = self.module_type_map.get(module_type_id).ok_or(FrameworkError::ModuleType_NotFound)?;

    Ok(module_type_entry.module_rust_type_name)
    }


    /// Check whether a module type has been registered.
    pub(crate) fn contains_module_type(&self, module_type_id: ModuleTypeID) -> bool {

        self.module_type_map.contains_key(module_type_id)
    }

}


