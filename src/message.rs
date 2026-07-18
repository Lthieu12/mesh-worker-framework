//! Message definitions for MMF.
//!
//! This file contains the structures exchanged between workers.
//! All communications within the framework are performed using
//! WorkerMessage instances.

use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::string_helper;
use crate::worker::{EntityID, WorkerID};

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

/// Identifies a unique message instance used for
/// tracing and request/reply correlation.
pub type MessageInstanceID = u64;

#[derive(Debug, Clone)]
/// Message exchanged between workers through MMF.
pub struct WorkerMessage {
    /// Identifier of the sender entity.
    pub sender_id: EntityID,

    /// Identifier of the receiver worker.
    pub receiver_id: WorkerID,

    /// Functional identifier of the message.
    pub message_id: MessageID,

    /// Unique identifier of this message instance.
    pub message_instance_id: MessageInstanceID,

    /// Binary payload associated with the message.
    pub message_data: Bytes,
}

impl WorkerMessage {
    /// Create a new worker message.
    pub(crate) fn new(sender_id: EntityID, receiver_id: WorkerID, message_id: MessageID, message_instance_id: MessageInstanceID, message_data: Bytes) -> Self {
        Self { sender_id, receiver_id, message_id, message_instance_id, message_data }
    }
}
