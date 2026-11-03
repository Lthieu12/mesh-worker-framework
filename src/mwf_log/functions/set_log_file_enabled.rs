use super::super::MWFLog;
use crate::MWF;
use crate::mwf::MWF_LOG_DATA;
use crate::mwf::MWFError;
use crate::mwf::MWFPanic;

impl MWFLog {
    /// Enables or disables writing captured log events to the configured file.
    ///
    /// The logging mode determines which worker events are captured by MWFLog.
    /// This setting independently controls whether captured events are also written
    /// to the log file.
    ///
    /// When enabled, each captured event is immediately written and flushed to disk.
    ///
    /// File logging is automatically disabled and the file handle is closed when
    /// the maximum file size is reached or a write operation fails.
    ///
    /// Once the file handle has been closed automatically, file logging cannot be
    /// re-enabled until MWF is initialized again.
    pub fn set_log_file_enabled(enabled: bool) -> Result<(), MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        let mut mwf_log_data_guard = MWF_LOG_DATA.write().expect(MWFPanic::MWFLogData_WriteLockPoisoned.message());

        let logging_data = mwf_log_data_guard.as_mut().ok_or(MWFError::Framework_NotInitialized)?;

        if enabled {
            if logging_data.log_file_path.is_none() {
                return Err(MWFError::Logging_LogFileNotConfigured);
            }

            if logging_data.log_file.is_none() {
                return Err(MWFError::Logging_LogFileAccessFailed(anyhow::anyhow!("The log file is no longer available. File logging cannot be re-enabled until MWF is initialized again.")));
            }
        }

        logging_data.log_file_enabled = enabled;

        Ok(())
    }
}
