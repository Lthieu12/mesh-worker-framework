use super::super::MWFWrk;
use crate::framework_error::MWFError;
use crate::framework_error::MWFPanic;
use crate::mwf::MWF;
use crate::mwf::MWF_WRK_DATA;
use crate::mwf::MWF_WRK_INIT_NOTIFY;
use crate::worker::WorkerID;
use crate::worker::WorkerState;
use crate::worker_task::current_worker_id;

impl MWFWrk {
    /// Wait until all workers in the list have reached the Initialized state.
    ///
    /// Duplicate WorkerIDs are ignored.
    ///
    /// Returns Worker_InitializationFailed if any worker failed during initialization.
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    ///
    /// A worker cannot wait for its own initialization.
    pub async fn wait_worker_list_init(worker_ids: &[WorkerID], timeout_ms: Option<u64>) -> Result<(), MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        let worker_ids = Self::deduplicate_worker_ids(worker_ids);

        // A worker cannot wait for its own initialization.
        if let Some(current_id) = current_worker_id() {
            if worker_ids.contains(&current_id) {
                return Err(MWFError::Worker_CannotWaitForSelf);
            }
        }

        let wait_future = async {
            loop {
                // Register interest in the next initialization event
                // before checking worker states to avoid lost notifications.
                let init_notify = MWF_WRK_INIT_NOTIFY.notified();
                tokio::pin!(init_notify);
                init_notify.as_mut().enable();

                // Read Registry state under a short-lived lock.
                //
                // The lock must be released before awaiting the
                // next notification.
                let (all_init_finished, one_init_failed) = {
                    let mwf_wrk_data_guard = MWF_WRK_DATA.read().expect(MWFPanic::MWFWrkData_ReadLockPoisoned.message());
                    let mwf_data = mwf_wrk_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;

                    let mut all_init_finished = true;
                    let mut one_init_failed = false;

                    // Initialization is complete only when all workers
                    // have reached the Initialized or InitFailed state.
                    for worker_id in &worker_ids {
                        match mwf_data.worker_registry.get_worker_state(worker_id)? {
                            WorkerState::Initialized => {}

                            WorkerState::InitFailed => {
                                one_init_failed = true;
                            }

                            _ => {
                                all_init_finished = false;
                            }
                        }
                    }

                    (all_init_finished, one_init_failed)
                };

                if all_init_finished {
                    if one_init_failed {
                        return Err(MWFError::Worker_InitializationFailed);
                    }

                    return Ok(());
                }

                // Wait for the next worker initialization event.
                init_notify.await;
            }
        };

        match timeout_ms {
            Some(timeout_ms) => tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), wait_future).await.map_err(|_| MWFError::Operation_Timeout)?,
            None => wait_future.await,
        }
    }
}
