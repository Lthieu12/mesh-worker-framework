use super::super::ErrorEvent;
use super::super::MWFErr;
use crate::MWF;
use crate::mwf::MWF_ERR_DATA;
use crate::mwf::MWF_ERR_INSTANCE_COUNTER;
use crate::mwf::MWF_ERR_NOTIFY;
use crate::mwf::MWFError;
use crate::mwf::MWFPanic;
use crate::mwf_wrk::types::EntityID;
use std::sync::atomic::Ordering;

impl MWFErr {
    /// Generate a unique ErrorInstanceID.
    fn generate_error_instance_id() -> Result<u64, MWFError> {
        MWF_ERR_INSTANCE_COUNTER.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current_id| current_id.checked_add(1)).map_err(|_| MWFError::Error_InstanceIDExhausted)
    }

    /// Record a new error event.
    ///
    /// This internal function is used by MWF to enrich error
    /// events with framework-specific context, such as the
    /// framework-specific context associated with the error.
    ///
    /// Application code should use `add_error()` instead.
    ///
    /// Error events are stored in a circular buffer and
    /// notify all MWFErr waiters.
    pub(crate) fn add_error_internal(entity_id: &EntityID, callback_name: Option<String>, error: &anyhow::Error) -> Result<(), MWFError> {
        let error_message = error.to_string();

        let mut error_context_list = Vec::new();

        // Record the MWF callback that generated the error.
        if let Some(callback_name) = callback_name {
            error_context_list.push(callback_name.to_string());
        }

        let mut context = error.source();
        while let Some(error_context) = context {
            error_context_list.push(error_context.to_string());
            context = error_context.source();
        }

        // Maintain the error circular buffer.
        //
        // When the configured capacity is reached,
        // the oldest error events are discarded to make
        // room for newer ones.
        let mut mwf_err_data_guard = MWF_ERR_DATA.write().expect(MWFPanic::MWFErrData_WriteLockPoisoned.message());
        let mwf_err_data = mwf_err_data_guard.as_mut().ok_or(MWFError::Framework_NotInitialized)?;

        let error_instance_id = MWFErr::generate_error_instance_id()?;
        let timestamp_rfc3339 = MWF::get_timestamp_rfc3339();
        let error_event = ErrorEvent { error_instance_id, timestamp_rfc3339, entity_id: entity_id.clone(), error_description: error_message, error_context_list };

        let error_event_buffer_capacity = mwf_err_data.error_event_buffer_capacity;

        while mwf_err_data.error_event_list.len() >= error_event_buffer_capacity {
            mwf_err_data.error_event_list.pop_front();
        }

        mwf_err_data.error_event_list.push_back(error_event);
        drop(mwf_err_data_guard);

        // Notify all waiters that a new error event
        // has been added to the buffer.
        MWF_ERR_NOTIFY.notify_waiters();

        Ok(())
    }
}
