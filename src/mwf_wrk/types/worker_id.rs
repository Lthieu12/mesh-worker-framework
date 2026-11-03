use serde::{Deserialize, Serialize};

use crate::mwf_wrk::types::EntityID;
use crate::string_helper;

/// Unique identifier of a worker instance.
///
/// MWF-generated WorkerIDs follow this format:
///
/// `WorkerTypeID:MWFInstanceID:WorkerCounter`
///
/// - `WorkerTypeID` identifies the worker type.
/// - `MWFInstanceID` identifies the MWF instance that created the worker.
/// - `WorkerCounter` identifies the worker instance within its worker type
///   and MWF instance.
///
/// The worker counter starts at 1 and is maintained independently for each
/// worker type within an MWF instance.
///
/// This format ensures that MWF-generated WorkerIDs are unique across
/// successive MWF instances within the same process.
///
/// WorkerIDs are not guaranteed to remain unique across process restarts.
/// Custom WorkerIDs created with `WorkerID::new()` are not required to
/// follow this format.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct WorkerID(String);

impl WorkerID {
    /// Creates a worker identifier.
    pub fn new(worker_id: impl Into<String>) -> Self {
        Self(worker_id.into())
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns a shortened version of the identifier while preserving the worker counter.
    pub fn to_short_string(&self, max_len: usize) -> String {
        let mut parts = self.as_str().rsplitn(3, ':');

        let counter = parts.next().unwrap_or_default();
        let mwf_instance_id = parts.next().unwrap_or_default();
        let worker_type_id = parts.next().unwrap_or_default();

        // Only apply the special formatting to generated worker IDs.
        let is_generated_id = !worker_type_id.is_empty() && !mwf_instance_id.is_empty() && mwf_instance_id.bytes().all(|byte| byte.is_ascii_digit()) && !counter.is_empty() && counter.bytes().all(|byte| byte.is_ascii_digit());

        if !is_generated_id {
            return string_helper::to_short_string(self.as_str(), max_len);
        }

        if self.as_str().len() <= max_len {
            return self.as_str().to_string();
        }

        // Preserve the final worker counter, including its separator.
        let suffix = format!(":{counter}");

        // The suffix and ellipsis must fit in the available width.
        if max_len <= suffix.len() + 3 {
            return string_helper::to_short_string(self.as_str(), max_len);
        }

        let prefix_len = max_len - suffix.len() - 3;
        let prefix = string_helper::truncate_prefix(self.as_str(), prefix_len);

        format!("{prefix}...{suffix}")
    }

    /// Returns this worker identifier as an EntityID.
    pub fn as_entity_id(&self) -> EntityID {
        EntityID::new_worker(self.clone())
    }
}

impl std::fmt::Display for WorkerID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for WorkerID {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for WorkerID {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for WorkerID {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&WorkerID> for WorkerID {
    fn from(value: &WorkerID) -> Self {
        value.clone()
    }
}
