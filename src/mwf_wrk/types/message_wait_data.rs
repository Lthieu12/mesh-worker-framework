//! Shared data used by the MWF message wait and flow registry.

use std::collections::HashMap;

use crate::mwf_wrk::types::{MessageFlowID, WorkerID};
use crate::wait_registry::{MessageFlowState, WorkerWaitState};

/// Internal data used to manage message waits and flows.
#[derive(Debug)]
pub(crate) struct MessageWaitData {
    /// Active message flows and their reply channels.
    pub(crate) message_flow_registry: HashMap<MessageFlowID, MessageFlowState>,

    /// Workers currently waiting for message completion.
    pub(crate) waiting_worker_registry: HashMap<WorkerID, WorkerWaitState>,
}

impl MessageWaitData {
    /// Create an empty message wait registry.
    pub(crate) fn new() -> Self {
        Self { message_flow_registry: HashMap::new(), waiting_worker_registry: HashMap::new() }
    }
}
