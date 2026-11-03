use super::super::MWFWrk;
use crate::mwf::MWF;
use crate::mwf::MWFError;
use crate::worker_task::current_message_inheritance_context;
use crate::worker_task::current_worker_context;
use crate::worker_task::scope_worker_context;

impl MWFWrk {
    /// Execute blocking work without blocking the MWF runtime.
    ///
    /// Typical uses include native DLL calls and CPU-intensive computations.
    ///
    /// This is a thin wrapper around [`tokio::task::spawn_blocking`].
    ///
    /// The blocking work is executed on Tokio's blocking thread pool,
    /// allowing the Tokio runtime to continue executing other MWF workers.
    pub async fn spawn_blocking<F, R>(f: F) -> Result<R, MWFError>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        let worker_context = current_worker_context();
        let message_context = current_message_inheritance_context();

        tokio::task::spawn_blocking(move || match worker_context {
            Some(worker_context) => scope_worker_context(worker_context, message_context, f),
            None => f(),
        })
        .await
        .map_err(|e| MWFError::BlockingTask_Failed(anyhow::anyhow!(e)))
    }
}
