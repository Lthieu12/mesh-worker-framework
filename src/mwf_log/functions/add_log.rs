use super::super::LogAction;
use super::super::LogEvent;
use super::super::LogMode;
use super::super::LogStatus;
use super::super::MWFLog;
use crate::MWF;
use crate::MWFErr;
use crate::log_file::write_log_event;
use crate::mwf::MWF_LOG_DATA;
use crate::mwf::MWF_LOG_INSTANCE_COUNTER;
use crate::mwf::MWF_LOG_MODE;
use crate::mwf::MWF_LOG_NOTIFY;
use crate::mwf::MWFError;
use crate::mwf::MWFPanic;
use crate::mwf_wrk::types::EntityID;
use crate::mwf_wrk::types::WorkerID;
use std::sync::atomic::Ordering;
use std::time::Instant;

impl MWFLog {
    /// Generate a unique LogInstanceID.
    fn generate_log_instance_id() -> Result<u64, MWFError> {
        MWF_LOG_INSTANCE_COUNTER.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current_id| current_id.checked_add(1)).map_err(|_| MWFError::Logging_InstanceIDExhausted)
    }

    /// Check whether logging is enabled for a worker.
    fn must_log_worker(worker_id: &WorkerID) -> Result<bool, MWFError> {
        let log_mode = MWF_LOG_MODE.load(Ordering::Relaxed);

        if log_mode == LogMode::NoLog as u8 {
            return Ok(false);
        }

        if log_mode == LogMode::LogAll as u8 {
            return Ok(true);
        }

        if log_mode == LogMode::LogSelected as u8 {
            let mwf_log_data_guard = MWF_LOG_DATA.read().expect(MWFPanic::MWFLogData_ReadLockPoisoned.message());
            let logging_data = mwf_log_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;
            return Ok(logging_data.logged_worker_list.iter().any(|logged_worker_id| logged_worker_id == worker_id));
        }

        // Invalid value => NoLog
        Ok(false)
    }

    /// Add a log event to MWFLog.
    ///
    /// If logging is disabled for the specified worker,
    /// the event is silently ignored.
    ///
    /// Returns Framework_NotInitialized if MWF has
    /// not been initialized through MWF::init().
    pub(crate) fn add_log(action_start_timer: Instant, worker_id: &WorkerID, log_action: LogAction, log_status: LogStatus) -> Result<(), MWFError> {
        // Skip immediately if logging is disabled
        // for this worker.
        if !Self::must_log_worker(worker_id)? {
            return Ok(());
        }

        let duration_ms: Option<u128> = match log_status {
            LogStatus::Start => None,

            LogStatus::Ok | LogStatus::Error => Some(action_start_timer.elapsed().as_millis()),
        };

        let mut mwf_log_data_guard = MWF_LOG_DATA.write().expect(MWFPanic::MWFLogData_WriteLockPoisoned.message());
        let logging_data = mwf_log_data_guard.as_mut().ok_or(MWFError::Framework_NotInitialized)?;

        let log_instance_id = MWFLog::generate_log_instance_id()?;
        let timestamp_rfc3339 = MWF::get_timestamp_rfc3339();
        let log_event = LogEvent { log_instance_id, timestamp_rfc3339, worker_id: worker_id.to_owned(), log_action, log_status, duration_ms };

        // Write the log event to the log file if enabled.
        let file_error = match write_log_event(logging_data, &log_event) {
            Ok(()) => None,

            Err(error) => {
                // Disable file logging without affecting memory logging.
                logging_data.log_file_enabled = false;
                logging_data.log_file = None;

                Some(error)
            }
        };

        // Circular buffer behavior.
        if logging_data.log_event_list.len() >= logging_data.log_event_buffer_capacity {
            logging_data.log_event_list.pop_front();
        }
        logging_data.log_event_list.push_back(log_event);

        drop(mwf_log_data_guard);

        // Notify all waiters that a new log event
        // has been added to the buffer.
        MWF_LOG_NOTIFY.notify_waiters();

        // Report file logging failures through MWFErr.
        if let Some(error) = file_error {
            let error = anyhow::Error::new(error);
            let _ = MWFErr::add_error_internal(&EntityID::Worker(worker_id.clone()), Some("MWFLog::add_log".into()), &error);
        }

        Ok(())
    }
}
