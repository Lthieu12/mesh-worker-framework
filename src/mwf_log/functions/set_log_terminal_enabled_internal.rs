use super::super::LogList;
use super::super::LogOutputFormat;
use super::super::MWFLog;
use crate::mwf::MWF_DATA;
use crate::mwf::MWF_INSTANCE_COUNTER;
use crate::mwf::MWF_LOG_DATA;
use crate::mwf::MWFError;
use crate::mwf::MWFPanic;
use std::sync::atomic::Ordering;
use tokio::sync::oneshot;

impl MWFLog {
    /// Enables or disables terminal logging for internal framework operations.
    ///
    /// Unlike the public API, this function does not validate the API context.
    /// It is intended for use by MWF internals, notably during framework shutdown.
    pub(crate) async fn set_log_terminal_enabled_internal(enabled: bool) -> Result<(), MWFError> {
        if enabled {
            // Prevent terminal logging from being enabled while MWF is shutting down.
            let mwf_data_guard = MWF_DATA.read().expect(MWFPanic::MWFData_ReadLockPoisoned.message());
            if mwf_data_guard.is_none() {
                return Err(MWFError::Framework_NotInitialized);
            }

            let mwf_instance_counter = MWF_INSTANCE_COUNTER.load(Ordering::Relaxed);

            // Create a one-shot channel used to delay log consumption until the
            // terminal logging task has been fully registered.
            let (start_sender, start_receiver) = oneshot::channel();

            // Create a one-shot channel used to request a graceful task shutdown.
            let (shutdown_sender, mut shutdown_receiver) = oneshot::channel();

            // Start the terminal logging task before acquiring the MWF_LOG_DATA lock.
            let task_handle = tokio::spawn(async move {
                // Wait until the terminal logging task has been fully registered.
                if start_receiver.await.is_err() {
                    return;
                }

                // Print the terminal log column header once when the logger starts.
                println!("{}", LogList::get_header(LogOutputFormat::Terminal));

                // Keep the last consumed log instance ID local to the logging task.
                let mut last_log_instance_id = None;

                loop {
                    tokio::select! {
                        log_result = MWFLog::wait_log_internal(mwf_instance_counter, last_log_instance_id) => {
                            match log_result {
                                Ok(log_list) => {
                                    // Print all newly received log events.
                                    for log_event in &log_list.log_event_list {
                                        println!("{}", log_event.to_terminal_string());
                                    }

                                    // Continue waiting from the last consumed log event.
                                    last_log_instance_id = log_list.last_log_instance_id;
                                }

                                Err(_) => {
                                    // Stop the terminal logging task if MWF is no longer
                                    // available.
                                    return;
                                }
                            }
                        }

                        _ = &mut shutdown_receiver => {
                            // Print all log events that remain in the MWF log buffer.
                            if let Ok(log_list) = MWFLog::get_log_list(mwf_instance_counter, last_log_instance_id) {
                                for log_event in &log_list.log_event_list {
                                    println!("{}", log_event.to_terminal_string());
                                }
                            }

                            return;
                        }
                    }
                }
            });

            // Register the terminal logging task before allowing it to consume logs.
            let mut mwf_log_data_guard = MWF_LOG_DATA.write().expect(MWFPanic::MWFLogData_WriteLockPoisoned.message());

            let logging_data = match mwf_log_data_guard.as_mut() {
                Some(logging_data) => logging_data,
                None => {
                    // The task is still waiting for its start signal, so it can be
                    // safely aborted because it has not started consuming logs.
                    task_handle.abort();

                    return Err(MWFError::Framework_NotInitialized);
                }
            };

            // Ignore redundant enable requests.
            if logging_data.log_terminal_enabled {
                // Another terminal logging task is already active.
                task_handle.abort();

                return Ok(());
            }

            logging_data.log_terminal_enabled = true;
            logging_data.log_terminal_task_handle = Some(task_handle);
            logging_data.log_terminal_shutdown_sender = Some(shutdown_sender);

            // The terminal logging task is now fully registered and can start
            // consuming log events.
            let _ = start_sender.send(());

            drop(mwf_data_guard);
        } else {
            // Retrieve the terminal logging task controls while updating its state.
            let (shutdown_sender, task_handle) = {
                let mut mwf_log_data_guard = MWF_LOG_DATA.write().expect(MWFPanic::MWFLogData_WriteLockPoisoned.message());

                let logging_data = mwf_log_data_guard.as_mut().ok_or(MWFError::Framework_NotInitialized)?;

                // Ignore redundant disable requests.
                if !logging_data.log_terminal_enabled {
                    return Ok(());
                }

                logging_data.log_terminal_enabled = false;

                (logging_data.log_terminal_shutdown_sender.take(), logging_data.log_terminal_task_handle.take())
            };

            // Request the terminal logging task to print its remaining log events
            // before terminating.
            if let Some(shutdown_sender) = shutdown_sender {
                let _ = shutdown_sender.send(());
            }

            // Wait until the terminal logging task has printed all remaining logs
            // and terminated.
            if let Some(task_handle) = task_handle {
                let _ = task_handle.await;
            }
        }

        Ok(())
    }
}
