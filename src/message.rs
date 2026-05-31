use bytes::Bytes;
use crate::module::ModuleID;
use crate::error::FrameworkError;

pub type MessageID = String;
pub type MessageInstanceID = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessagePriority {
    Normal,
    High,
}

pub struct FrameworkMessage {
    pub sender_id: ModuleID,
    pub receiver_id: ModuleID,
    pub message_id: MessageID,
    pub message_instance_id: MessageInstanceID,
    pub message_priority: MessagePriority,
    pub message_data: Bytes,
}