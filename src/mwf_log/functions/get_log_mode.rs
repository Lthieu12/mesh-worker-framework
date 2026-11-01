use super::super::LogMode;
use super::super::MWFLog;
use crate::MWF;
use crate::framework_error::MWFError;
use crate::framework_error::MWFPanic;
use crate::mwf::MWF_LOG_DATA;
use crate::mwf::MWF_LOG_MODE;
use std::sync::atomic::Ordering;

impl MWFLog {
    /// Retrieve the current MWF logging mode.
    ///
    /// Returns Framework_NotInitialized if MWF has not
    /// been initialized through MWF::init().
    pub fn get_log_mode() -> Result<LogMode, MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        let mwf_log_data_guard = MWF_LOG_DATA.read().expect(MWFPanic::MWFLogData_ReadLockPoisoned.message());

        if mwf_log_data_guard.is_none() {
            return Err(MWFError::Framework_NotInitialized);
        }

        let log_mode = match MWF_LOG_MODE.load(Ordering::Relaxed) {
            0 => LogMode::NoLog,
            1 => LogMode::LogAll,
            2 => LogMode::LogSelected,
            // Invalid value => NoLog
            _ => LogMode::NoLog,
        };

        drop(mwf_log_data_guard);
        Ok(log_mode)
    }
}
