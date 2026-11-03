use super::super::LogInstanceID;
use super::super::LogList;
use super::super::MWFLog;
use crate::mwf::MWF_LOG_NOTIFY;
use crate::mwf::MWFError;

impl MWFLog {
    pub(crate) async fn wait_log_internal(mwf_instance_counter: u64, last_log_instance_id: Option<LogInstanceID>) -> Result<LogList, MWFError> {
        loop {
            let log_notify = MWF_LOG_NOTIFY.notified();
            tokio::pin!(log_notify);
            log_notify.as_mut().enable();

            let log_list = MWFLog::get_log_list(mwf_instance_counter, last_log_instance_id)?;

            if !log_list.log_event_list.is_empty() {
                return Ok(log_list);
            }
            log_notify.await;
        }
    }
}
