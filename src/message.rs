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

/// Defines the priority of a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessagePriority {

    /// Standard message processing.
    Normal,

    /// High priority message processing.
    High,
}

#[derive(Debug)]
/// Message exchanged between modules through MMF.
pub struct FrameworkMessage {

    /// Identifier of the sender module.
    pub sender_id: EntityID,

    /// Identifier of the receiver module.
    pub receiver_id: ModuleID,

    /// Functional identifier of the message.
    pub message_id: MessageID,

    /// Unique identifier of this message instance.
    pub message_instance_id: MessageInstanceID,

    /// Processing priority of the message.
    pub message_priority: MessagePriority,

    /// Binary payload associated with the message.
    pub message_data: Bytes,
}