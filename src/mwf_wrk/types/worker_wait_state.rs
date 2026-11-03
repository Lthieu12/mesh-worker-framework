use std::time::Instant;

use crate::mwf_wrk::types::{MessageID, MessageInstanceID, MessageWaitMode, WorkerID};

/// Information about a worker currently waiting for message completion.
#[derive(Debug)]
pub(crate) struct WorkerWaitState {
    /// Worker receiving the awaited message.
    pub(crate) receiver_id: WorkerID,

    /// Functional identifier of the awaited message.
    pub(crate) message_id: MessageID,

    /// Unique identifier of the awaited message instance.
    pub(crate) message_instance_id: MessageInstanceID,

    /// Type of completion being awaited.
    pub(crate) _wait_mode: MessageWaitMode,

    /// Time when the wait started.
    pub(crate) _started_at: Instant,
}
