use super::super::MWFWrk;
use crate::framework_error::MWFError;
use crate::worker::WorkerID;

impl MWFWrk {
    /// Wait until a worker is no longer registered in MWF.
    ///
    /// See wait_worker_list_close() for details.
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    pub async fn wait_worker_close(worker_id: &WorkerID, timeout_ms: Option<u64>) -> Result<(), MWFError> {
        MWFWrk::wait_worker_list_close(std::slice::from_ref(worker_id), timeout_ms).await
    }
}
