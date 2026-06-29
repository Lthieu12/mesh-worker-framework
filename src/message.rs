//! Message definitions for MMF.
//!
//! This module contains the structures exchanged between modules.
//! All communications within the framework are performed using
//! FrameworkMessage instances.

use bytes::Bytes;
use crate::module::{ModuleID, EntityID};

/// Identifies the functional purpose of a message
/// (e.g. "Start", "Stop", "AcquisitionDone").
pub type MessageID = String;

/// Identifies a unique message instance used for
/// tracing and request/reply correlation.
pub type MessageInstanceID = u64;


#[derive(Debug, Clone)]
/// Message exchanged between modules through MMF.
pub struct ModuleMessage {

    /// Identifier of the sender entity.
    pub sender_id: EntityID,

    /// Identifier of the receiver module.
    pub receiver_id: ModuleID,

    /// Functional identifier of the message.
    pub message_id: MessageID,

    /// Unique identifier of this message instance.
    pub message_instance_id: MessageInstanceID,

    /// Binary payload associated with the message.
    pub message_data: Bytes,
}

impl ModuleMessage {

    /// Create a new module message.
    pub(crate) fn new(sender_id: EntityID, receiver_id: ModuleID, message_id: MessageID, message_instance_id: MessageInstanceID, message_data: Bytes) -> Self {

        Self {
            sender_id,
            receiver_id,
            message_id,
            message_instance_id,
            message_data,
        }
    }
}

