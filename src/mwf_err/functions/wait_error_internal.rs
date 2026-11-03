use super::super::ErrorInstanceID;
use super::super::ErrorList;
use super::super::MWFErr;
use crate::mwf::MWF_ERR_NOTIFY;
use crate::mwf::MWFError;

impl MWFErr {
    pub(crate) async fn wait_error_internal(mwf_instance_counter: u64, last_error_instance_id: Option<ErrorInstanceID>) -> Result<ErrorList, MWFError> {
        loop {
            let error_notify = MWF_ERR_NOTIFY.notified();
            tokio::pin!(error_notify);
            error_notify.as_mut().enable();

            let error_list = MWFErr::get_error_list(mwf_instance_counter, last_error_instance_id)?;

            if !error_list.error_event_list.is_empty() {
                return Ok(error_list);
            }
            error_notify.await;
        }
    }
}
