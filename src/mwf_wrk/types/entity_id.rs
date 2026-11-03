use serde::{Deserialize, Serialize};

use crate::mwf_wrk::types::{ExternalID, WorkerID};
use crate::string_helper;

/// Identifier of an entity interacting with MWF.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityID {
    /// MWF worker entity.
    Worker(WorkerID),
    /// External entity not managed by MWF.
    External(ExternalID),
}

impl std::fmt::Display for EntityID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityID::Worker(worker_id) => {
                write!(f, "{worker_id}")
            }

            EntityID::External(external_id) => {
                write!(f, "External:{external_id}")
            }
        }
    }
}

impl EntityID {
    /// Creates a worker entity.
    pub fn new_worker(worker_id: WorkerID) -> Self {
        Self::Worker(worker_id)
    }

    /// Creates an external entity.
    pub fn new_external(external_id: impl Into<ExternalID>) -> Self {
        Self::External(external_id.into())
    }

    /// Returns true if the identifier is valid.
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Worker(_) => true,
            Self::External(external_id) => external_id.is_valid(),
        }
    }

    /// Returns a shortened human-readable representation of the entity.
    ///
    /// Worker entities preserve the worker counter whenever possible.
    /// External entities are truncated at the end.
    pub(crate) fn to_short_string(&self, max_len: usize) -> String {
        match self {
            EntityID::Worker(worker_id) => worker_id.to_short_string(max_len),
            EntityID::External(external_id) => string_helper::to_short_string(external_id.as_str(), max_len),
        }
    }
}
