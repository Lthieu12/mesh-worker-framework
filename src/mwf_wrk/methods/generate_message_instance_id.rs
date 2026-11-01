use super::super::MWFWrk;
use crate::framework_error::MWFError;
use crate::message::MessageInstanceID;
use crate::mwf::MWF_WRK_MESSAGE_INSTANCE_COUNTER;
use std::sync::atomic::Ordering;

impl MWFWrk {
    /// Generate a unique MessageInstanceID.
    pub(crate) fn generate_message_instance_id() -> Result<MessageInstanceID, MWFError> {
        MWF_WRK_MESSAGE_INSTANCE_COUNTER.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current_id| current_id.checked_add(1)).map_err(|_| MWFError::Message_InstanceIDExhausted)
    }
}
