use super::super::MWFWrk;
use std::time::Duration;

impl MWFWrk {
    /// Suspend the current worker for the specified duration.
    ///
    /// This is a thin wrapper around [`tokio::time::sleep`].
    ///
    /// While this worker is suspended, the Tokio runtime can continue
    /// executing other MWF workers.
    pub async fn sleep(duration: Duration) {
        tokio::time::sleep(duration).await;
    }
}
