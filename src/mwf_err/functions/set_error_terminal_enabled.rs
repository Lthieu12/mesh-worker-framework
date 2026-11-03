use super::super::MWFErr;
use crate::MWF;
use crate::mwf::MWFError;

impl MWFErr {
    /// Enables or disables MWF terminal error logging.
    ///
    /// When enabled, MWF starts a background task that waits for new error events
    /// and prints them to the terminal.
    ///
    /// When disabled, the terminal error logging task is requested to stop, prints any
    /// remaining error events, and is awaited until it has fully terminated.
    pub async fn set_error_terminal_enabled(enabled: bool) -> Result<(), MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        MWFErr::set_error_terminal_enabled_internal(enabled).await
    }
}
