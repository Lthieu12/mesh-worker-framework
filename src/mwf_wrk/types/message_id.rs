use serde::{Deserialize, Serialize};

use crate::string_helper;

/// Identifies the functional purpose of a message
/// (e.g. "Start", "Stop", "AcquisitionDone").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct MessageID(String);

impl MessageID {
    /// Creates a message identifier.
    pub fn new(message_id: impl Into<String>) -> Self {
        Self(message_id.into())
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
    // Validation only rejects identifiers that would break MWF itself
    // (logging, formatting, diagnostics, etc.).
    // Application-specific naming rules are intentionally left to the user.
    pub fn is_valid(&self) -> bool {
        let id = self.0.trim();
        !id.is_empty() && !id.contains('\n') && !id.contains('\r')
    }
}

impl std::fmt::Display for MessageID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for MessageID {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for MessageID {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for MessageID {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&MessageID> for MessageID {
    fn from(value: &MessageID) -> Self {
        value.clone()
    }
}
