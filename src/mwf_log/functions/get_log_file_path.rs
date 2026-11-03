use super::super::MWFLog;
use crate::MWF;
use crate::mwf::MWF_LOG_DATA;
use crate::mwf::MWFError;
use crate::mwf::MWFPanic;
use std::path::PathBuf;

impl MWFLog {
    /// Returns the current MWF log file path.
    pub fn get_log_file_path() -> Result<Option<PathBuf>, MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        let mwf_log_data_guard = MWF_LOG_DATA.read().expect(MWFPanic::MWFLogData_ReadLockPoisoned.message());

        let logging_data = mwf_log_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;

        Ok(logging_data.log_file_path.clone())
    }
}
