//! Public MWF API.
//!
//! This API exposes the framework entry points used to
//! initialize and shutdown the API global runtime.

use chrono::{Local, SecondsFormat, Utc};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::{LazyLock, RwLock, RwLockReadGuard};
use tokio::sync::Notify;

use crate::MWFErr;
use crate::MWFLog;
use crate::framework_error::{MWFError, MWFPanic};
use crate::log_file;
use crate::message_wait::MessageWaitData;
use crate::mwf::TimestampFormat::LocalTime;
use crate::mwf_err::ErrorData;
use crate::mwf_log::{LogMode, LoggingData};
use crate::mwf_wrk::WorkerData;
use crate::worker::{Worker, WorkerConstructor};
use crate::worker_task::current_worker_id;
use crate::worker_type_registry::WorkerTypeRegistry;

/// Global MWF instance counter.
pub(crate) static MWF_INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(0);
pub(crate) type MWFInstanceID = u64;

/// Global MWF runtime storage shared by all
/// framework components.
pub(crate) static MWF_DATA: LazyLock<RwLock<Option<MWFData>>> = LazyLock::new(|| RwLock::new(None));

/// Global MWF worker runtime storage.
pub(crate) static MWF_WRK_DATA: LazyLock<RwLock<Option<WorkerData>>> = LazyLock::new(|| RwLock::new(None));

/// Global MWF notifier used to signal child worker initialized.
pub(crate) static MWF_WRK_INIT_NOTIFY: LazyLock<Notify> = LazyLock::new(Notify::new);

/// Global MWF notifier used to signal child worker closures.
pub(crate) static MWF_WRK_CLOSE_NOTIFY: LazyLock<Notify> = LazyLock::new(Notify::new);

/// Global MWF message instance counter used to identify messages sent to workers.
pub(crate) static MWF_WRK_MESSAGE_INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Global MWF message flow counter used to identify message flows.
pub(crate) static MWF_WRK_MESSAGE_FLOW_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Global MWF error event storage.
pub(crate) static MWF_ERR_DATA: LazyLock<RwLock<Option<ErrorData>>> = LazyLock::new(|| RwLock::new(None));

/// MWF error notification emitted whenever a new error event
/// is added to MWFErr.
pub(crate) static MWF_ERR_NOTIFY: LazyLock<Notify> = LazyLock::new(Notify::new);

/// Global MWF error instance counter.
pub(crate) static MWF_ERR_INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Global logging storage.
pub(crate) static MWF_LOG_DATA: LazyLock<RwLock<Option<LoggingData>>> = LazyLock::new(|| RwLock::new(None));

/// Current logging mode.
pub(crate) static MWF_LOG_MODE: AtomicU8 = AtomicU8::new(LogMode::NoLog as u8);

/// MWF log notification emitted whenever a new log event
/// is added to MWFLog.
pub(crate) static MWF_LOG_NOTIFY: LazyLock<Notify> = LazyLock::new(Notify::new);

/// Global log event counter.
pub(crate) static MWF_LOG_INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Current timestamp format used by the framework.
pub(crate) static MWF_TIMESTAMP_FORMAT: AtomicU8 = AtomicU8::new(TimestampFormat::LocalTime as u8);

/// Global MWF message wait and flow storage.
pub(crate) static MWF_MESSAGE_WAIT_DATA: LazyLock<RwLock<Option<MessageWaitData>>> = LazyLock::new(|| RwLock::new(None));

/// Timestamp format used by MWF.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimestampFormat {
    LocalTime = 0,
    UtcTime = 1,
}

/// Information about the Mesh Worker Framework.
#[derive(Debug, Clone, Copy)]
pub struct MWFInfo {
    /// crate name.
    pub crate_name: &'static str,

    /// crate version.
    pub crate_version: &'static str,

    /// Git commit used to build the crate.
    pub crate_git_commit: &'static str,

    /// Build profile ("debug" or "release").
    pub build_profile: &'static str,
}

impl std::fmt::Display for MWFInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        writeln!(f, "===============================")?;
        writeln!(f, "     {}", self.crate_name.to_uppercase())?;
        writeln!(f, "===============================")?;
        writeln!(f, "Version : {}", self.crate_version)?;
        writeln!(f, "Commit  : {}", self.crate_git_commit)?;
        writeln!(f, "Profile : {}", self.build_profile)?;
        writeln!(f, "===============================")
    }
}

/// Internal MWF runtime data.
#[derive(Debug)]
pub(crate) struct MWFData {
    /// Registered worker types.
    pub(crate) worker_type_registry: WorkerTypeRegistry,
}

impl MWFData {
    pub(crate) fn new() -> Self {
        Self { worker_type_registry: WorkerTypeRegistry::new() }
    }
}

/// MWF startup configuration.
#[derive(Debug, Clone)]
pub struct MWFConfiguration {
    /// Capacity of normal worker event queues (must be >= 1).
    pub worker_normal_channel_capacity: usize,

    /// Capacity of priority worker event queues (must be >= 1).
    pub worker_priority_channel_capacity: usize,

    /// Timestamp format used by MWF.
    pub timestamp_format: TimestampFormat,

    /// Capacity of the MWFErr circular buffer (must be >= 1).
    pub error_event_buffer_capacity: usize,

    /// Capacity of the MWFLog circular buffer (must be >= 1).
    pub log_event_buffer_capacity: usize,

    /// Path to the MWF log file.
    ///
    /// By default, MWF creates a log file next to the executable.
    /// Set to `None` to disable file logging and prevent the log file from being created.
    pub log_file_path: Option<PathBuf>,

    /// Maximum log file size in bytes before file logging is disabled (must be >= 1024).
    pub log_file_max_size_bytes: u64,
}

impl Default for MWFConfiguration {
    fn default() -> Self {
        let mut log_file_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("mwf.exe"));
        log_file_path.set_file_name("mwf.log");

        let log_file_max_size_bytes = 10 * 1024 * 1024;

        Self { worker_normal_channel_capacity: 50, worker_priority_channel_capacity: 10, timestamp_format: LocalTime, error_event_buffer_capacity: 100, log_event_buffer_capacity: 100, log_file_path: Some(log_file_path), log_file_max_size_bytes }
    }
}

/// Public MWF entry point.
pub struct MWF;

impl MWF {
    /// Returns information about the Mesh Worker Framework.
    pub fn get_info() -> MWFInfo {
        MWFInfo { crate_name: env!("CARGO_PKG_NAME"), crate_version: env!("CARGO_PKG_VERSION"), crate_git_commit: env!("MWF_GIT_COMMIT"), build_profile: if cfg!(debug_assertions) { "debug" } else { "release" } }
    }

    /// Initialize MWF using the default configuration.
    ///
    /// Each successful initialization after a previous shutdown receives a new
    /// instance ID, used to ensure WorkerID uniqueness within the current process.
    pub fn init() -> Result<(), MWFError> {
        Self::init_with_configuration(MWFConfiguration::default())
    }

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
        let mut message_wait_data_guard = MWF_MESSAGE_WAIT_DATA.write().expect(MWFPanic::MWFMessageWaitData_WriteLockPoisoned.message());
        *message_wait_data_guard = Some(MessageWaitData::new());
        drop(message_wait_data_guard);

        Ok(())
    }

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

    /// Check whether the framework is initialized.
    pub fn is_initialized() -> bool {
        MWF_DATA.read().expect(MWFPanic::MWFData_ReadLockPoisoned.message()).is_some()
    }

    /// Validate that MWF is initialized and that the current worker is still registered.
    ///
    /// Calls made outside an MWF worker task only check framework initialization.
    ///
    /// Returns the worker data read guard so callers can reuse it.
    pub(crate) fn validate_api_context() -> Result<RwLockReadGuard<'static, Option<WorkerData>>, MWFError> {
        // Check framework initialization.
        if !MWF::is_initialized() {
            return Err(MWFError::Framework_NotInitialized);
        }

        // Acquire worker data and validate the current worker when applicable.
        let mwf_wrk_data_guard = MWF_WRK_DATA.read().expect(MWFPanic::MWFWrkData_ReadLockPoisoned.message());
        let mwf_wrk_data = mwf_wrk_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;

        if let Some(worker_id) = current_worker_id() {
            mwf_wrk_data.worker_registry.get_registry_entry(&worker_id)?;
        }

        Ok(mwf_wrk_data_guard)
    }

    /// Register a worker type.
    ///
    /// worker types must be registered before they can be
    /// instantiated by MWF.
    ///
    /// Registration associates the worker's WorkerTypeID
    /// with:
    /// - its Rust type name, used for diagnostics;
    /// - a constructor function used by MWF to create new
    ///   worker instances dynamically.
    ///
    /// The registered constructor creates a new default
    /// instance of the worker each time MWF spawns it.
    ///
    /// Returns:
    /// - Framework_NotInitialized if MWF has not been initialized.
    /// - WorkerType_AlreadyRegistered if the worker type has
    ///   already been registered.
    /// - Worker_DuplicateWorkerTypeID if the WorkerTypeID is
    ///   already associated with a different Rust worker type.
    pub fn register_worker_type<TWorker>() -> Result<(), MWFError>
    where
        TWorker: Worker + Send + 'static,
    {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        let mut mwf_data_guard = MWF_DATA.write().expect(MWFPanic::MWFData_WriteLockPoisoned.message());
        let mwf_data = mwf_data_guard.as_mut().ok_or(MWFError::Framework_NotInitialized)?;

        let worker_type_id = TWorker::worker_type_id();
        if !worker_type_id.is_valid() {
            return Err(MWFError::WorkerType_Invalid { worker_type_id });
        }

        let rust_type_name = std::any::type_name::<TWorker>();

        match mwf_data.worker_type_registry.get_worker_rust_type_name(&TWorker::worker_type_id()) {
            Ok(registered_rust_type_name) => {
                if registered_rust_type_name == rust_type_name {
                    return Err(MWFError::WorkerType_AlreadyRegistered);
                }

                return Err(MWFError::WorkerType_DuplicateWorkerTypeID);
            }

            Err(MWFError::WorkerType_NotFound) => {
                // Create the constructor associated with this worker type.
                //
                // The constructor is stored in the WorkerTypeRegistry and
                // invoked by MWF whenever a new instance of this worker
                // type must be spawned.
                //
                // Note that T::new() is not executed here.
                // Instead, a function calling T::new() is created and
                // stored for later use.
                let worker_constructor: WorkerConstructor = || Box::new(TWorker::new());
                mwf_data.worker_type_registry.add_worker_type(&TWorker::worker_type_id(), rust_type_name, worker_constructor);
            }

            Err(error) => return Err(error),
        }

        Ok(())
    }

    /// Returns the current timestamp formatted according to RFC3339.
    ///
    /// The timestamp is generated using the current MWF timestamp
    /// configuration (local time or UTC).
    pub fn get_timestamp_rfc3339() -> String {
        match MWF_TIMESTAMP_FORMAT.load(Ordering::Relaxed) {
            x if x == TimestampFormat::LocalTime as u8 => Local::now().to_rfc3339_opts(SecondsFormat::Millis, false),
            x if x == TimestampFormat::UtcTime as u8 => Utc::now().to_rfc3339_opts(SecondsFormat::Millis, false),
            _ => Local::now().to_rfc3339_opts(SecondsFormat::Millis, false),
        }
    }
}
