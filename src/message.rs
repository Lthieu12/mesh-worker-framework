//! Message definitions for MWF.
//!
//! This file contains the structures exchanged between workers.
//! All communications within the framework are performed using
//! WorkerMessage instances.

use serde::{Deserialize, Serialize};
use std::any::Any;

use crate::MWFError;
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

pub struct MessageData {
    /// Stored message data.
    value: Box<dyn Any + Send + Sync>,

    /// Fully qualified Rust type name of the stored message data.
    type_name: &'static str,
}

impl MessageData {
    /// Creates a new MessageData by cloning the provided value.
    pub fn new<T>(value: &T) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        Self { value: Box::new(value.clone()), type_name: std::any::type_name::<T>() }
    }

    /// Returns true if the stored message data is of type `T`.
    pub fn is<T>(&self) -> bool
    where
        T: Send + Sync + 'static,
    {
        self.value.is::<T>()
    }

    /// Returns the fully qualified Rust type name of the stored message data.
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// Consumes this MessageData and returns the stored value as type `T`.
    pub fn into<T>(self) -> Result<T, MWFError>
    where
        T: Send + Sync + 'static,
    {
        // Retrieve the expected type name.
        let expected_type = std::any::type_name::<T>();

        // Downcast the stored value to the requested type.
        let value = self.value.downcast::<T>().map_err(|_| MWFError::Message_DataTypeMismatch { expected_type, actual_type: self.type_name })?;

        Ok(*value)
    }
}

impl std::fmt::Debug for MessageData {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("MessageData").field("type_name", &self.type_name).finish()
    }
}

#[derive(Debug)]
/// Message exchanged between workers through MWF.
pub struct WorkerMessage {
    /// Identifier of the sender entity.
    pub sender_id: EntityID,

    /// Identifier of the receiver worker.
    pub receiver_id: WorkerID,

    /// Functional identifier of the message.
    pub message_id: MessageID,

    /// Unique identifier of this message instance.
    pub message_instance_id: MessageInstanceID,

    /// Data associated with the message.
    pub message_data: MessageData,
}

impl WorkerMessage {
    /// Create a new worker message.
    pub(crate) fn new<TMessageData>(sender_id: &EntityID, receiver_id: &WorkerID, message_id: &MessageID, message_instance_id: MessageInstanceID, message_data: &TMessageData) -> Self
    where
        TMessageData: Clone + Send + Sync + 'static,
    {
        Self { sender_id: sender_id.clone(), receiver_id: receiver_id.clone(), message_id: message_id.clone(), message_instance_id, message_data: MessageData::new(message_data) }
    }
}
