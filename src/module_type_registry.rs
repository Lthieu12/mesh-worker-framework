//! Module type registry for MMF.
//!
//! This module stores all module types known by MMF.
//!
//! Registered module types are identified by a ModuleTypeID
//! and associated with their Rust module type.

use std::collections::HashMap;

use crate::framework_error::FrameworkError;
use crate::module::{ModuleConstructor, ModuleTypeID};

/// Information associated with a registered module type.
#[derive(Debug)]
pub(crate) struct ModuleTypeEntry {
    /// Fully qualified Rust type name.
    pub(crate) module_rust_type_name: &'static str,

    /// Function creating a new instance of this module type.
    pub(crate) module_constructor: ModuleConstructor,
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
        Self { module_type_map: HashMap::new() }
    }

    /// Register a module type.
    pub(crate) fn add_module_type(&mut self, module_type_id: ModuleTypeID, module_rust_type_name: &'static str, module_constructor: ModuleConstructor) {
        self.module_type_map.insert(module_type_id, ModuleTypeEntry { module_rust_type_name, module_constructor });
    }

    /// Get the Rust type name associated with a ModuleTypeID.
    pub(crate) fn get_module_rust_type_name(&self, module_type_id: ModuleTypeID) -> Result<&'static str, FrameworkError> {
        let module_type_entry = self.module_type_map.get(module_type_id).ok_or(FrameworkError::ModuleType_NotFound)?;
        Ok(module_type_entry.module_rust_type_name)
    }

    /// Returns the constructor associated with a ModuleTypeID.
    ///
    /// The returned function creates a new instance of the
    /// corresponding module type each time it is invoked.
    pub(crate) fn get_module_constructor(&self, module_type_id: ModuleTypeID) -> Result<ModuleConstructor, FrameworkError> {
        let module_type_entry = self.module_type_map.get(module_type_id).ok_or(FrameworkError::ModuleType_NotFound)?;
        Ok(module_type_entry.module_constructor)
    }

    /// Check whether a module type has been registered.
    pub(crate) fn contains_module_type(&self, module_type_id: ModuleTypeID) -> bool {
        self.module_type_map.contains_key(module_type_id)
    }
}
