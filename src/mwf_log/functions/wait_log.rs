use super::super::LogInstanceID;
use super::super::LogList;
use super::super::MWFLog;
use crate::MWF;
use crate::framework_error::MWFError;
use crate::mwf::MWF_INSTANCE_COUNTER;
use std::sync::atomic::Ordering;

impl MWFLog {
    /// Wait until new log events are available for the current MWF instance.
    ///
    /// If last_log_instance_id is None, all log events currently stored in the
    /// current MWF instance are returned.
    ///
    /// If last_log_instance_id is provided, only events with a greater
    /// LogInstanceID are returned.
    ///
    /// If matching events are already available, they are returned immediately.
    /// Otherwise, the function waits for the next log notification.
    ///
    /// Each wait is bound to the MWF instance in which it starts. If that
    /// instance is closed while the wait is pending, including when MWF is
    /// reinitialized before the wait resumes, the function returns Ok(None).
    /// It never switches to the newly initialized instance.
    ///
    /// LogInstanceID values restart for each MWF instance. After reinitializing
    /// MWF, start a new retrieval sequence with None instead of reusing an ID
    /// from the previous instance.
    ///
    /// Returns Framework_NotInitialized if called when MWF is already closed.
    pub async fn wait_log(last_log_instance_id: Option<LogInstanceID>) -> Result<Option<LogList>, MWFError> {
        let mwf_wrk_data_guard = MWF::validate_api_context()?;
        let mwf_instance_counter = MWF_INSTANCE_COUNTER.load(Ordering::Relaxed);
        drop(mwf_wrk_data_guard);

        match MWFLog::wait_log_internal(mwf_instance_counter, last_log_instance_id).await {
            Ok(log_list) => Ok(Some(log_list)),
            Err(MWFError::Framework_NotInitialized) => Ok(None),
            Err(error) => Err(error),
        }
    }
}
