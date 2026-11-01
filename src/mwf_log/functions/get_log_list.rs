use super::super::LogInstanceID;
use super::super::LogList;
use super::super::MWFLog;
use crate::framework_error::MWFError;
use crate::framework_error::MWFPanic;
use crate::mwf::MWF_INSTANCE_COUNTER;
use crate::mwf::MWF_LOG_DATA;
use std::sync::atomic::Ordering;

impl MWFLog {
    /// Retrieve log events stored in MWFLog.
    ///
    /// If last_log_instance_id is None,
    /// all events currently present in the circular
    /// buffer are returned.
    ///
    /// If last_log_instance_id is provided,
    /// only events with a greater LogInstanceID are
    /// returned.
    ///
    /// missing_log_count indicates how many events
    /// could not be returned because they were overwritten
    /// in the circular buffer before being retrieved.
    ///
    /// Returns Framework_NotInitialized if MWFLog has
    /// not been initialized through MWF::init().
    pub(crate) fn get_log_list(mwf_instance_counter: u64, last_log_instance_id: Option<LogInstanceID>) -> Result<LogList, MWFError> {
        let mwf_log_data_guard = MWF_LOG_DATA.read().expect(MWFPanic::MWFLogData_ReadLockPoisoned.message());
        let mwf_log_data = mwf_log_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;

        // Make sure the log buffer still belongs to the original MWF instance.
        if MWF_INSTANCE_COUNTER.load(Ordering::Relaxed) != mwf_instance_counter {
            return Err(MWFError::Framework_NotInitialized);
        }

        // No log event is currently stored in MWFLog.
        if mwf_log_data.log_event_list.is_empty() {
            return Ok(LogList { first_log_instance_id: None, last_log_instance_id: None, missing_log_count: 0, log_event_list: Vec::new() });
        }

        // Retrieve the current boundaries of the circular buffer.
        //
        // The first and last LogInstanceID are used to detect
        // whether older events have been overwritten before
        // the caller could retrieve them.
        let first_log_event = mwf_log_data.log_event_list.front().expect(MWFPanic::LogBuffer_Inconsistency.message());
        let last_log_event = mwf_log_data.log_event_list.back().expect(MWFPanic::LogBuffer_Inconsistency.message());

        let first_buffer_log_instance_id = first_log_event.log_instance_id;
        let last_buffer_log_instance_id = last_log_event.log_instance_id;

        // Determine how many events were missed by the caller.
        //
        // Missing events occur when the caller requests events
        // older than the first LogInstanceID currently present
        // in the circular buffer.
        let missing_log_count = match last_log_instance_id {
            None => 0,

            Some(last_seen) => {
                if last_seen >= last_buffer_log_instance_id {
                    0
                } else if last_seen < first_buffer_log_instance_id {
                    first_buffer_log_instance_id.saturating_sub(last_seen + 1)
                } else {
                    0
                }
            }
        };

        // Return only the events that have not yet been
        // received by the caller.
        let log_event_list = match last_log_instance_id {
            None => mwf_log_data.log_event_list.iter().cloned().collect(),

            Some(last_seen) => mwf_log_data.log_event_list.iter().filter(|log_event| log_event.log_instance_id > last_seen).cloned().collect(),
        };

        Ok(LogList { first_log_instance_id: Some(first_buffer_log_instance_id), last_log_instance_id: Some(last_buffer_log_instance_id), missing_log_count, log_event_list })
    }
}
