use super::super::ErrorInstanceID;
use super::super::ErrorList;
use super::super::MWFErr;
use crate::mwf::MWF_ERR_DATA;
use crate::mwf::MWF_INSTANCE_COUNTER;
use crate::mwf::MWFError;
use crate::mwf::MWFPanic;
use std::sync::atomic::Ordering;

impl MWFErr {
    /// Retrieve error events stored in MWFErr.
    ///
    /// If last_error_instance_id is None,
    /// all events currently present in the circular
    /// buffer are returned.
    ///
    /// If last_error_instance_id is provided,
    /// only events with a greater ErrorInstanceID are
    /// returned.
    ///
    /// missing_error_count indicates how many events
    /// could not be returned because they were overwritten
    /// in the circular buffer before being retrieved.
    ///
    /// Returns Framework_NotInitialized if MWFErr has
    /// not been initialized through MWF::init().
    pub(crate) fn get_error_list(mwf_instance_counter: u64, last_error_instance_id: Option<ErrorInstanceID>) -> Result<ErrorList, MWFError> {
        let mwf_err_data_guard = MWF_ERR_DATA.read().expect(MWFPanic::MWFErrData_ReadLockPoisoned.message());
        let mwf_err_data = mwf_err_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;

        // Make sure the error buffer still belongs to the original MWF instance.
        if MWF_INSTANCE_COUNTER.load(Ordering::Relaxed) != mwf_instance_counter {
            return Err(MWFError::Framework_NotInitialized);
        }

        // No error event is currently stored in MWFErr.
        if mwf_err_data.error_event_list.is_empty() {
            return Ok(ErrorList { first_error_instance_id: None, last_error_instance_id: None, missing_error_count: 0, error_event_list: Vec::new() });
        }

        // Retrieve the current boundaries of the circular buffer.
        //
        // The first and last ErrorInstanceID are used to detect
        // whether older events have been overwritten before
        // the caller could retrieve them.
        let first_error_event = mwf_err_data.error_event_list.front().expect(MWFPanic::ErrorBuffer_Inconsistency.message());
        let last_error_event = mwf_err_data.error_event_list.back().expect(MWFPanic::ErrorBuffer_Inconsistency.message());

        let first_buffer_error_instance_id = first_error_event.error_instance_id;
        let last_buffer_error_instance_id = last_error_event.error_instance_id;

        // Determine how many events were missed by the caller.
        //
        // Missing events occur when the caller requests events
        // older than the first ErrorInstanceID currently present
        // in the circular buffer.
        let missing_error_count = match last_error_instance_id {
            None => 0,

            Some(last_seen) => {
                if last_seen >= last_buffer_error_instance_id {
                    0
                } else if last_seen < first_buffer_error_instance_id {
                    first_buffer_error_instance_id.saturating_sub(last_seen + 1)
                } else {
                    0
                }
            }
        };

        // Return only the events that have not yet been
        // received by the caller.
        let error_event_list = match last_error_instance_id {
            None => mwf_err_data.error_event_list.iter().cloned().collect(),

            Some(last_seen) => mwf_err_data.error_event_list.iter().filter(|error_event| error_event.error_instance_id > last_seen).cloned().collect(),
        };

        Ok(ErrorList { first_error_instance_id: Some(first_buffer_error_instance_id), last_error_instance_id: Some(last_buffer_error_instance_id), missing_error_count, error_event_list })
    }
}
