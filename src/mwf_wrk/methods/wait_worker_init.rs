use super::super::MWFWrk;
use crate::framework_error::MWFError;
use crate::worker::WorkerID;

impl MWFWrk {
    /// Wait until a worker reaches the Initialized state.
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    pub async fn wait_worker_init(worker_id: &WorkerID, timeout_ms: Option<u64>) -> Result<(), MWFError> {
        MWFWrk::wait_worker_list_init(std::slice::from_ref(worker_id), timeout_ms).await
    }
}
