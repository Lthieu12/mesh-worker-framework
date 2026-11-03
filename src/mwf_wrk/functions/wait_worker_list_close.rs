use super::super::MWFWrk;
use crate::mwf::MWF;
use crate::mwf::MWF_WRK_CLOSE_NOTIFY;
use crate::mwf::MWF_WRK_DATA;
use crate::mwf::MWFError;
use crate::mwf::MWFPanic;
use crate::mwf_wrk::types::WorkerID;
use crate::mwf_wrk::types::WorkerState;
use crate::worker_task::current_worker_id;

impl MWFWrk {
    /// Wait until all workers in the list are no longer registered in MWF.
    ///
    /// Duplicate WorkerIDs are ignored.
    ///
    /// During a normal shutdown, a worker completes its lifecycle, including on_close(),
    /// before being removed from the Registry.
    ///
    /// When aborted, a worker is removed from the Registry without waiting for its Tokio
    /// task to terminate. Tasks blocked in synchronous code or native DLL calls may
    /// continue executing.
    ///
    /// Returns Worker_CloseFailed if a worker failed during on_close().
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    ///
    /// A worker cannot wait for its own closure.
    pub async fn wait_worker_list_close(worker_ids: &[WorkerID], timeout_ms: Option<u64>) -> Result<(), MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        let worker_ids = Self::deduplicate_worker_ids(worker_ids);

        // A worker cannot wait for its own closure.
        if let Some(current_id) = current_worker_id() {
            if worker_ids.contains(&current_id) {
                return Err(MWFError::Worker_CannotWaitForSelf);
            }
        }

        let wait_future = async {
            loop {
                // Register interest in the next closing event
                // before checking worker states to avoid lost notifications.
                let close_notify = MWF_WRK_CLOSE_NOTIFY.notified();
                tokio::pin!(close_notify);
                close_notify.as_mut().enable();

                // Read Registry state under a short-lived lock.
                //
                // The lock must be released before awaiting the
                // next notification.
                let (all_close_finished, one_close_failed) = {
                    let mwf_wrk_data_guard = MWF_WRK_DATA.read().expect(MWFPanic::MWFWrkData_ReadLockPoisoned.message());
                    let mwf_data = mwf_wrk_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;

                    let mut all_close_finished = true;
                    let mut one_close_failed = false;

                    // Closing is complete only when all workers
                    // have either been removed from the Registry
                    // or reached the CloseFailed or the Panicked state.
                    for worker_id in &worker_ids {
                        match mwf_data.worker_registry.get_worker_state(worker_id) {
                            // A worker not found has been properly closed.
                            Err(MWFError::Worker_NotFound) => {}

                            Ok(WorkerState::CloseFailed) => {
                                one_close_failed = true;
                            }

                            _ => {
                                all_close_finished = false;
                            }
                        }
                    }

                    (all_close_finished, one_close_failed)
                };

                if all_close_finished {
                    if one_close_failed {
                        return Err(MWFError::Worker_CloseFailed);
                    }

                    return Ok(());
                }

                close_notify.await;
            }
        };

        match timeout_ms {
            Some(timeout_ms) => tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), wait_future).await.map_err(|_| MWFError::Operation_Timeout)?,
            None => wait_future.await,
        }
    }
}
