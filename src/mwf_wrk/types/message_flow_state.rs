use tokio::sync::oneshot;

use crate::mwf::MWFError;
use crate::mwf_wrk::types::WorkerID;

/// Runtime state associated with an active message flow.
#[derive(Debug)]
pub(crate) struct MessageFlowState {
    /// Worker that owns and executes this message flow.
    pub(crate) worker_id: WorkerID,

    /// Channel used to return the flow result to its waiter.
    pub(crate) reply_sender: oneshot::Sender<Result<(), MWFError>>,
}
