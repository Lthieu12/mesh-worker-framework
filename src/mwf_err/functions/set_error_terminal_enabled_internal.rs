use super::super::ErrorList;
use super::super::ErrorOutputFormat;
use super::super::MWFErr;
use crate::framework_error::MWFError;
use crate::framework_error::MWFPanic;
use crate::mwf::MWF_DATA;
use crate::mwf::MWF_ERR_DATA;
use crate::mwf::MWF_INSTANCE_COUNTER;
use std::sync::atomic::Ordering;
use tokio::sync::oneshot;

impl MWFErr {
    /// Enables or disables terminal error logging for internal framework operations.
    ///
    /// Unlike the public API, this function does not validate the API context.
    /// It is intended for use by MWF internals, notably during framework shutdown.
    pub(crate) async fn set_error_terminal_enabled_internal(enabled: bool) -> Result<(), MWFError> {
        if enabled {
            // Prevent terminal error logging from being enabled while MWF is shutting down.
            let mwf_data_guard = MWF_DATA.read().expect(MWFPanic::MWFData_ReadLockPoisoned.message());
            if mwf_data_guard.is_none() {
                return Err(MWFError::Framework_NotInitialized);
            }

            let mwf_instance_counter = MWF_INSTANCE_COUNTER.load(Ordering::Relaxed);

            // Create a one-shot channel used to delay error consumption until the
            // terminal error logging task has been fully registered.
            let (start_sender, start_receiver) = oneshot::channel();

            // Create a one-shot channel used to request a graceful task shutdown.
            let (shutdown_sender, mut shutdown_receiver) = oneshot::channel();

            // Start the terminal error logging task before acquiring the MWF_ERR_DATA lock.
            let task_handle = tokio::spawn(async move {
                // Wait until the terminal error logging task has been fully registered.
                if start_receiver.await.is_err() {
                    return;
                }

                // Print the terminal error column header once when the logger starts.
                println!("{}", ErrorList::get_header(ErrorOutputFormat::Terminal));

                // Keep the last consumed error instance ID local to the logging task.
                let mut last_error_instance_id = None;

                loop {
                    tokio::select! {
                        error_result = MWFErr::wait_error_internal(mwf_instance_counter, last_error_instance_id) => {
                            match error_result {
                                Ok(error_list) => {
                                    // Print all newly received error events.
                                    for error_event in &error_list.error_event_list {
                                        println!("{}", error_event.to_terminal_string());
                                    }

                                    // Continue waiting from the last consumed error event.
                                    last_error_instance_id = error_list.last_error_instance_id;
                                }

                                Err(_) => {
                                    // Stop the terminal error logging task if MWF is no longer
                                    // available.
                                    return;
                                }
                            }
                        }

                        _ = &mut shutdown_receiver => {
                            // Print all error events that remain in the MWF error buffer.
                            if let Ok(error_list) = MWFErr::get_error_list(mwf_instance_counter, last_error_instance_id) {
                                for error_event in &error_list.error_event_list {
                                    println!("{}", error_event.to_terminal_string());
                                }
                            }

                            return;
                        }
                    }
                }
            });

            // Register the terminal error logging task before allowing it to consume errors.
            let mut mwf_err_data_guard = MWF_ERR_DATA.write().expect(MWFPanic::MWFErrData_WriteLockPoisoned.message());

            let error_data = match mwf_err_data_guard.as_mut() {
                Some(error_data) => error_data,
                None => {
                    // The task is still waiting for its start signal, so it can be
                    // safely aborted because it has not started consuming errors.
                    task_handle.abort();

                    return Err(MWFError::Framework_NotInitialized);
                }
            };

            // Ignore redundant enable requests.
            if error_data.error_terminal_enabled {
                // Another terminal error logging task is already active.
                task_handle.abort();

                return Ok(());
            }

            error_data.error_terminal_enabled = true;
            error_data.error_terminal_task_handle = Some(task_handle);
            error_data.error_terminal_shutdown_sender = Some(shutdown_sender);

            // The terminal error logging task is now fully registered and can start
            // consuming error events.
            let _ = start_sender.send(());

            drop(mwf_data_guard);
        } else {
            // Retrieve the terminal error logging task controls while updating its state.
            let (shutdown_sender, task_handle) = {
                let mut mwf_err_data_guard = MWF_ERR_DATA.write().expect(MWFPanic::MWFErrData_WriteLockPoisoned.message());

                let error_data = mwf_err_data_guard.as_mut().ok_or(MWFError::Framework_NotInitialized)?;

                // Ignore redundant disable requests.
                if !error_data.error_terminal_enabled {
                    return Ok(());
                }

                error_data.error_terminal_enabled = false;

                (error_data.error_terminal_shutdown_sender.take(), error_data.error_terminal_task_handle.take())
            };

            // Request the terminal error logging task to print its remaining error events
            // before terminating.
            if let Some(shutdown_sender) = shutdown_sender {
                let _ = shutdown_sender.send(());
            }

            // Wait until the terminal error logging task has printed all remaining errors
            // and terminated.
            if let Some(task_handle) = task_handle {
                task_handle.await.map_err(|error| MWFError::BlockingTask_Failed(anyhow::anyhow!(error)))?;
            }
        }

        Ok(())
    }
}
