use super::super::MWF;
use super::super::MWF_DATA;
use super::super::MWF_ERR_DATA;
use super::super::MWF_ERR_INSTANCE_COUNTER;
use super::super::MWF_INSTANCE_COUNTER;
use super::super::MWF_LOG_DATA;
use super::super::MWF_LOG_INSTANCE_COUNTER;
use super::super::MWF_LOG_MODE;
use super::super::MWF_TIMESTAMP_FORMAT;
use super::super::MWF_WRK_DATA;
use super::super::MWF_WRK_MESSAGE_FLOW_COUNTER;
use super::super::MWF_WRK_MESSAGE_INSTANCE_COUNTER;
use super::super::MWF_WRK_MESSAGE_WAIT_DATA;
use super::super::MWFConfiguration;
use super::super::MWFData;
use crate::log_file;
use crate::mwf::MWFError;
use crate::mwf::MWFPanic;
use crate::mwf_err::ErrorData;
use crate::mwf_log::LogMode;
use crate::mwf_log::LoggingData;
use crate::mwf_wrk::WorkerData;
use crate::mwf_wrk::types::MessageWaitData;
use crate::worker_task::current_worker_id;
use std::collections::VecDeque;
use std::sync::atomic::Ordering;

impl MWF {
    /// Initialize the MWF framework with custom configuration.
    ///
    /// Each successful initialization after a previous shutdown receives a new
    /// instance ID, used to ensure WorkerID uniqueness within the current process.
    pub fn init_with_configuration(mut mwf_configuration: MWFConfiguration) -> Result<(), MWFError> {
        // Verify that this function is not called from an MWF worker context.
        if current_worker_id().is_some() {
            return Err(MWFError::Framework_FunctionNotAvailableFromWorkerContext);
        }

        // Verify that MWF is running on a multi-threaded Tokio runtime.
        let runtime_handle = match tokio::runtime::Handle::try_current() {
            Ok(runtime_handle) => runtime_handle,
            Err(_) => return Err(MWFError::Framework_InvalidTokioRuntime),
        };
        if runtime_handle.runtime_flavor() != tokio::runtime::RuntimeFlavor::MultiThread {
            return Err(MWFError::Framework_InvalidTokioRuntime);
        }

        // Verify that MWF is not already initialized.
        let mut mwf_data_guard = MWF_DATA.write().expect(MWFPanic::MWFData_WriteLockPoisoned.message());
        if mwf_data_guard.is_some() {
            return Err(MWFError::Framework_AlreadyInitialized);
        }
        // Verify that supplied configuration is valid.
        if mwf_configuration.worker_normal_channel_capacity == 0 || mwf_configuration.worker_priority_channel_capacity == 0 || mwf_configuration.error_event_buffer_capacity == 0 || mwf_configuration.log_event_buffer_capacity == 0 || mwf_configuration.log_file_max_size_bytes < 1024 {
            return Err(MWFError::Framework_InvalidConfiguration);
        }
        // Initialize the MWF log file if file logging is enabled.
        let log_file = if mwf_configuration.log_file_path.is_some() { Some(log_file::init_log_file(&mut mwf_configuration)?) } else { None };

        *mwf_data_guard = Some(MWFData::new());

        // Generate a unique identifier for this MWF instance.
        // This prevents workers from previous instances from being mistaken for current workers.
        let mwf_instance_id = MWF_INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed);

        MWF_WRK_MESSAGE_INSTANCE_COUNTER.store(1, Ordering::Relaxed);
        MWF_WRK_MESSAGE_FLOW_COUNTER.store(1, Ordering::Relaxed);
        MWF_ERR_INSTANCE_COUNTER.store(1, Ordering::Relaxed);
        MWF_LOG_INSTANCE_COUNTER.store(1, Ordering::Relaxed);
        MWF_TIMESTAMP_FORMAT.store(mwf_configuration.timestamp_format as u8, Ordering::Relaxed);
        MWF_LOG_MODE.store(LogMode::NoLog as u8, Ordering::Relaxed);

        let mut mwf_wrk_data_guard = MWF_WRK_DATA.write().expect(MWFPanic::MWFWrkData_WriteLockPoisoned.message());
        *mwf_wrk_data_guard = Some(WorkerData::new(&mwf_configuration, mwf_instance_id));
        drop(mwf_wrk_data_guard);

        let mut mwf_err_data_guard = MWF_ERR_DATA.write().expect(MWFPanic::MWFErrData_WriteLockPoisoned.message());
        *mwf_err_data_guard = Some(ErrorData { error_event_buffer_capacity: mwf_configuration.error_event_buffer_capacity, error_event_list: VecDeque::new(), error_terminal_enabled: false, error_terminal_shutdown_sender: None, error_terminal_task_handle: None });
        drop(mwf_err_data_guard);

        // Initialize the MWF log file.
        let mut mwf_log_data_guard = MWF_LOG_DATA.write().expect(MWFPanic::MWFLogData_WriteLockPoisoned.message());
        *mwf_log_data_guard = Some(LoggingData {
            log_event_buffer_capacity: mwf_configuration.log_event_buffer_capacity,
            logged_worker_list: Vec::new(),
            log_event_list: VecDeque::new(),
            log_file_enabled: false,
            log_file_path: mwf_configuration.log_file_path,
            log_file,
            log_file_max_size_bytes: mwf_configuration.log_file_max_size_bytes,
            log_file_current_size_bytes: 0,
            log_terminal_enabled: false,
            log_terminal_shutdown_sender: None,
            log_terminal_task_handle: None,
        });
        drop(mwf_log_data_guard);

        // Initialize message wait and flow data.
        let mut message_wait_data_guard = MWF_WRK_MESSAGE_WAIT_DATA.write().expect(MWFPanic::MWFMessageWaitData_WriteLockPoisoned.message());
        *message_wait_data_guard = Some(MessageWaitData::new());
        drop(message_wait_data_guard);

        Ok(())
    }
}
