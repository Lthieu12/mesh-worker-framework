use super::super::MWFWrk;
use crate::mwf::MWF_WRK_MESSAGE_INSTANCE_COUNTER;
use crate::mwf::MWFError;
use crate::mwf_wrk::types::MessageInstanceID;
use std::sync::atomic::Ordering;

impl MWFWrk {
    /// Generate a unique MessageInstanceID.
    pub(crate) fn generate_message_instance_id() -> Result<MessageInstanceID, MWFError> {
        MWF_WRK_MESSAGE_INSTANCE_COUNTER.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current_id| current_id.checked_add(1)).map_err(|_| MWFError::Message_InstanceIDExhausted)
    }
}
