use crate::mwf_wrk::types::EntityID;
use crate::mwf_wrk::types::{MessageID, MessageInstanceID};

/// Logged MWF action.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LogAction {
    WorkerSpawn,
    WorkerInit,
    WorkerClose,
    WorkerAbort,
    WorkerPanic,
    WorkerMessage { message_instance_id: MessageInstanceID, sender_id: EntityID, message_id: MessageID, priority: bool, wait_completion: bool },
}
