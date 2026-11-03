use super::super::MWFWrk;
use std::time::Duration;

impl MWFWrk {
    /// Suspend the current asynchronous task for the specified duration.
    ///
    /// This is a thin wrapper around [`tokio::time::sleep`].
    ///
    /// While the task is suspended, the Tokio runtime can continue
    /// executing other tasks, including MWF workers.
    pub async fn sleep(duration: Duration) {
        tokio::time::sleep(duration).await;
    }
}
