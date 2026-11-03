use super::super::ErrorInstanceID;
use super::super::ErrorList;
use super::super::MWFErr;
use crate::MWF;
use crate::mwf::MWF_INSTANCE_COUNTER;
use crate::mwf::MWFError;
use std::sync::atomic::Ordering;

impl MWFErr {
    /// Wait until new error events are available for the current MWF instance.
    ///
    /// If last_error_instance_id is None, all error events currently stored in the
    /// current MWF instance are returned.
    ///
    /// If last_error_instance_id is provided, only events with a greater
    /// ErrorInstanceID are returned.
    ///
    /// If matching events are already available, they are returned immediately.
    /// Otherwise, the function waits for the next error notification.
    ///
    /// Each wait is bound to the MWF instance in which it starts. If that
    /// instance is closed while the wait is pending, including when MWF is
    /// reinitialized before the wait resumes, the function returns Ok(None).
    /// It never switches to the newly initialized instance.
    ///
    /// ErrorInstanceID values restart for each MWF instance. After reinitializing
    /// MWF, start a new retrieval sequence with None instead of reusing an ID
    /// from the previous instance.
    ///
    /// Returns Framework_NotInitialized if called when MWF is already closed.
    pub async fn wait_error(last_error_instance_id: Option<ErrorInstanceID>) -> Result<Option<ErrorList>, MWFError> {
        let mwf_wrk_data_guard = MWF::validate_api_context()?;
        let mwf_instance_counter = MWF_INSTANCE_COUNTER.load(Ordering::Relaxed);
        drop(mwf_wrk_data_guard);

        match MWFErr::wait_error_internal(mwf_instance_counter, last_error_instance_id).await {
            Ok(error_list) => Ok(Some(error_list)),
            Err(MWFError::Framework_NotInitialized) => Ok(None),
            Err(error) => Err(error),
        }
    }
}
