//! worker type registry for MWF.
//!
//! This file stores all worker types known by MWF.
//!
//! Registered worker types are identified by a WorkerTypeID
//! and associated with their Rust worker type.

use std::collections::HashMap;

use crate::framework_error::MWFError;
use crate::worker::{WorkerConstructor, WorkerTypeID};

/// Information associated with a registered worker type.
#[derive(Debug)]
pub(crate) struct WorkerTypeEntry {
    /// Fully qualified Rust type name.
    pub(crate) worker_rust_type_name: &'static str,

    /// Function creating a new instance of this worker type.
    pub(crate) worker_constructor: WorkerConstructor,
}

/// Stores all worker types known by the framework.
#[derive(Debug)]
pub(crate) struct WorkerTypeRegistry {
    /// Maps a WorkerTypeID to its associated Rust worker type.
    worker_type_map: HashMap<WorkerTypeID, WorkerTypeEntry>,
}

impl WorkerTypeRegistry {
    /// Create an empty worker type registry.
    pub(crate) fn new() -> Self {
        Self { worker_type_map: HashMap::new() }
    }

    /// Register a worker type.
    pub(crate) fn add_worker_type(&mut self, worker_type_id: WorkerTypeID, worker_rust_type_name: &'static str, worker_constructor: WorkerConstructor) {
        self.worker_type_map.insert(worker_type_id, WorkerTypeEntry { worker_rust_type_name, worker_constructor });
    }

    /// Get the Rust type name associated with a WorkerTypeID.
    pub(crate) fn get_worker_rust_type_name(&self, worker_type_id: WorkerTypeID) -> Result<&'static str, MWFError> {
        let worker_type_entry = self.worker_type_map.get(worker_type_id).ok_or(MWFError::WorkerType_NotFound)?;
        Ok(worker_type_entry.worker_rust_type_name)
    }

    /// Returns the constructor associated with a WorkerTypeID.
    ///
    /// The returned function creates a new instance of the
    /// corresponding worker type each time it is invoked.
    pub(crate) fn get_worker_constructor(&self, worker_type_id: WorkerTypeID) -> Result<WorkerConstructor, MWFError> {
        let worker_type_entry = self.worker_type_map.get(worker_type_id).ok_or(MWFError::WorkerType_NotFound)?;
        Ok(worker_type_entry.worker_constructor)
    }

    /// Check whether a worker type has been registered.
    pub(crate) fn contains_worker_type(&self, worker_type_id: WorkerTypeID) -> bool {
        self.worker_type_map.contains_key(worker_type_id)
    }
}
