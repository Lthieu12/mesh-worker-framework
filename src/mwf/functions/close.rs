use super::super::MWF;
use super::super::MWF_DATA;
use super::super::MWF_ERR_DATA;
use super::super::MWF_ERR_NOTIFY;
use super::super::MWF_LOG_DATA;
use super::super::MWF_LOG_NOTIFY;
use super::super::MWF_MESSAGE_WAIT_DATA;
use super::super::MWF_WRK_DATA;
use crate::MWFErr;
use crate::MWFLog;
use crate::mwf::MWFError;
use crate::mwf::MWFPanic;
use crate::worker_task::current_worker_id;

impl MWF {
    /// Shutdown the MWF framework and release all resources.
    ///
    /// All workers must be closed before calling this function.
    ///
    /// If one or more workers are still running,
    /// MWFError::Framework_CannotClose_WorkersStillRunning
    /// is returned.
    ///
    /// Use MWFWrk::close_worker() to terminate workers
    /// gracefully, or MWFWrk::abort_worker() as a last
    /// resort if a worker becomes unresponsive.
    pub fn close() -> Result<(), MWFError> {
        // Verify that this function is not called from an MWF worker context.
        if current_worker_id().is_some() {
            return Err(MWFError::Framework_FunctionNotAvailableFromWorkerContext);
        }

        // Acquire exclusive access to prevent concurrent shutdown or initialization.
        let mut mwf_data_guard = MWF_DATA.write().expect(MWFPanic::MWFData_WriteLockPoisoned.message());

        if mwf_data_guard.is_none() {
            return Err(MWFError::Framework_NotInitialized);
        }

        // Make sure no worker is still running.
        {
            let mwf_wrk_data_guard = MWF_WRK_DATA.read().expect(MWFPanic::MWFWrkData_ReadLockPoisoned.message());
            let mwf_wrk_data = mwf_wrk_data_guard.as_ref().expect(MWFPanic::MWFWrkData_Missing.message());
            if !mwf_wrk_data.worker_registry.is_empty() {
                return Err(MWFError::Framework_CannotClose_WorkersStillRunning);
            }
        }

        // Disable terminal logging and wait for the terminal logging task to terminate.
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(async { MWFLog::set_log_terminal_enabled_internal(false).await }))?;
        // Disable terminal error logging and wait for the terminal error task to terminate.
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(async { MWFErr::set_error_terminal_enabled_internal(false).await }))?;

        // Acquire all framework data write locks before destroying any data.
        let mut mwf_wrk_data_guard = MWF_WRK_DATA.write().expect(MWFPanic::MWFWrkData_WriteLockPoisoned.message());
        let mut mwf_err_data_guard = MWF_ERR_DATA.write().expect(MWFPanic::MWFErrData_WriteLockPoisoned.message());
        let mut mwf_log_data_guard = MWF_LOG_DATA.write().expect(MWFPanic::MWFLogData_WriteLockPoisoned.message());
        let mut message_wait_data_guard = MWF_MESSAGE_WAIT_DATA.write().expect(MWFPanic::MWFMessageWaitData_WriteLockPoisoned.message());

        // Make sure no worker has been spawned during the shutdown sequence.
        let mwf_wrk_data = mwf_wrk_data_guard.as_ref().expect(MWFPanic::MWFWrkData_Missing.message());
        if !mwf_wrk_data.worker_registry.is_empty() {
            return Err(MWFError::Framework_CannotClose_WorkersStillRunning);
        }

        // Destroy all framework data.
        *mwf_data_guard = None;
        *mwf_wrk_data_guard = None;
        *mwf_err_data_guard = None;
        *mwf_log_data_guard = None;
        *message_wait_data_guard = None;

        // Wake up all tasks waiting for log or error events.
        MWF_LOG_NOTIFY.notify_waiters();
        MWF_ERR_NOTIFY.notify_waiters();

        Ok(())
    }
}
