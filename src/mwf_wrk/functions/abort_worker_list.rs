use super::super::MWFWrk;
use crate::MWFLog;
use crate::framework_error::MWFError;
use crate::framework_error::MWFPanic;
use crate::mwf::MWF;
use crate::mwf::MWF_WRK_CLOSE_NOTIFY;
use crate::mwf::MWF_WRK_DATA;
use crate::mwf::MWF_WRK_INIT_NOTIFY;
use crate::mwf_log::LogAction;
use crate::mwf_log::LogStatus;
use crate::worker::WorkerID;
use std::time::Instant;

impl MWFWrk {
    /// Aborts a list of workers.
    ///
    /// Duplicate WorkerIDs are ignored; each unique worker is processed once.
    ///
    /// This is a best-effort operation intended as a last resort when workers cannot be closed
    /// normally. It requests immediate task cancellation through Tokio and removes the workers
    /// from the Registry.
    ///
    /// It does not wait for task termination. Tasks blocked in synchronous code or native DLL
    /// calls may continue executing. MWF-managed resources are cleaned up as far as possible,
    /// but resources owned by code that remains blocked cannot be guaranteed to be released.
    ///
    /// Each registered worker is expected to have an associated JoinHandle. A missing handle
    /// is an internal invariant violation and causes a panic.
    ///
    /// Errors encountered while processing the list do not prevent the remaining workers from
    /// being processed. The first encountered error is returned after all workers have been
    /// processed. Abort events are also recorded in MWFLog when logging is enabled.
    ///
    /// If an aborted worker continues executing after MWF has been reinitialized, it may still
    /// emit log and error events into the current MWF instance. These events retain the original
    /// WorkerID, including the MWF instance ID, so their origin remains identifiable.
    pub fn abort_worker_list(worker_ids: &[WorkerID]) -> Result<(), MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        let worker_ids = Self::deduplicate_worker_ids(worker_ids);

        let mut first_error: Option<MWFError> = None;
        let mut worker_action_list = Vec::with_capacity(worker_ids.len());

        let mut mwf_data_guard = MWF_WRK_DATA.write().expect(MWFPanic::MWFWrkData_WriteLockPoisoned.message());
        let mwf_data = mwf_data_guard.as_mut().ok_or(MWFError::Framework_NotInitialized)?;

        // Request cancellation of all worker tasks.
        for worker_id in &worker_ids {
            let action_start_timer = Instant::now();

            match mwf_data.worker_registry.take_join_handle(worker_id) {
                Ok(join_handle) => {
                    // Request immediate cancellation without waiting for task termination.
                    join_handle.abort();
                }
                Err(error) => {
                    // Keep processing the remaining workers.
                    let _ = MWFLog::add_log(action_start_timer, worker_id, LogAction::WorkerAbort, LogStatus::Error);

                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
            }

            worker_action_list.push((worker_id.clone(), action_start_timer));
        }

        // Remove all targeted workers from the Registry, regardless of cancellation results.
        for (worker_id, action_start_timer) in &worker_action_list {
            match mwf_data.worker_registry.remove_worker(worker_id) {
                Ok(()) => {
                    let _ = MWFLog::add_log(*action_start_timer, worker_id, LogAction::WorkerAbort, LogStatus::Ok);
                    let _ = MWFLog::remove_logged_worker(worker_id);
                }
                Err(error) => {
                    let _ = MWFLog::add_log(*action_start_timer, worker_id, LogAction::WorkerAbort, LogStatus::Error);

                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
            }
        }

        drop(mwf_data_guard);

        // Wake up all waiters interested in worker initialization and shutdown events.
        MWF_WRK_CLOSE_NOTIFY.notify_waiters();
        MWF_WRK_INIT_NOTIFY.notify_waiters();

        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}
