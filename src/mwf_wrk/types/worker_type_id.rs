use crate::string_helper;

/// Unique identifier of a worker type.
///
/// A WorkerTypeID identifies a worker *type*, not a worker instance.
///
/// Worker type identifiers are typically defined as compile-time
/// constants by each worker implementation, but they can also be
/// reconstructed at runtime when loading persisted workflow
/// definitions.
///
/// Rules:
/// - Must be unique across the whole application.
/// - Should remain stable between software versions if persisted.
/// - Is case-sensitive.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WorkerTypeID(String);

impl WorkerTypeID {
    /// Creates a worker type identifier.
    pub fn new(worker_type_id: impl Into<String>) -> Self {
        Self(worker_type_id.into())
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns a shortened version of the identifier.
    pub fn to_short_string(&self, max_len: usize) -> String {
        string_helper::to_short_string(self.as_str(), max_len)
    }

    /// Returns true if the identifier is valid.
    ///
    /// Validation only rejects identifiers that would break MWF itself
    /// (logging, formatting, diagnostics, etc.).
    /// Application-specific naming rules are intentionally left to the user.
    pub fn is_valid(&self) -> bool {
        let id = self.0.trim();
        !id.is_empty() && !id.contains('\n') && !id.contains('\r')
    }
}

impl std::fmt::Display for WorkerTypeID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for WorkerTypeID {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for WorkerTypeID {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for WorkerTypeID {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&WorkerTypeID> for WorkerTypeID {
    fn from(value: &WorkerTypeID) -> Self {
        value.clone()
    }
}
