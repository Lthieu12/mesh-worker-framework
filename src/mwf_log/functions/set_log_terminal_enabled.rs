use super::super::MWFLog;
use crate::MWF;
use crate::mwf::MWFError;

impl MWFLog {
    /// Enables or disables MWF terminal logging.
    ///
    /// When enabled, MWF starts a background task that waits for new log events
    /// and prints them to the terminal.
    ///
    /// When disabled, the terminal logging task is requested to stop, prints any
    /// remaining log events, and is awaited until it has fully terminated.
    pub async fn set_log_terminal_enabled(enabled: bool) -> Result<(), MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        MWFLog::set_log_terminal_enabled_internal(enabled).await
    }
}
