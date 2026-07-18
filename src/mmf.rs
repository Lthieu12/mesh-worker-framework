//! Public MMF API.
//!
//! This API exposes the framework entry points used to
//! initialize and shutdown the API global runtime.

use chrono::{Local, SecondsFormat, Utc};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::{LazyLock, RwLock};
use tokio::sync::Notify;

use crate::framework_error::{MMFError, MMFPanic};
use crate::log_file;
use crate::mmf::TimestampFormat::LocalTime;
use crate::mmf_err::ErrorData;
use crate::mmf_log::{LogMode, LoggingData};
use crate::mmf_wrk::WorkerData;
use crate::worker::{Worker, WorkerConstructor};
use crate::worker_type_registry::WorkerTypeRegistry;

/// Global MMF runtime storage shared by all
/// framework components.
pub(crate) static MMF_DATA: LazyLock<RwLock<Option<MMFData>>> = LazyLock::new(|| RwLock::new(None));

/// Global MMF worker runtime storage.
pub(crate) static MMF_WRK_DATA: LazyLock<RwLock<Option<WorkerData>>> = LazyLock::new(|| RwLock::new(None));

/// Global MMF notifier used to signal child worker initialized.
pub(crate) static MMF_WRK_INIT_NOTIFY: LazyLock<Notify> = LazyLock::new(Notify::new);

/// Global MMF notifier used to signal child worker closures.
pub(crate) static MMF_WRK_CLOSE_NOTIFY: LazyLock<Notify> = LazyLock::new(Notify::new);

/// Global MMF message instance counter used to identify messages sent to workers.
pub(crate) static MMF_WRK_MESSAGE_INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Global MMF error event storage.
pub(crate) static MMF_ERR_DATA: LazyLock<RwLock<Option<ErrorData>>> = LazyLock::new(|| RwLock::new(None));

/// MMF error notification emitted whenever a new error event
/// is added to MMFErr.
pub(crate) static MMF_ERR_NOTIFY: LazyLock<Notify> = LazyLock::new(Notify::new);

/// Global MMF error instance counter.
pub(crate) static MMF_ERR_INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Global logging storage.
pub(crate) static MMF_LOG_DATA: LazyLock<RwLock<Option<LoggingData>>> = LazyLock::new(|| RwLock::new(None));

/// Current logging mode.
pub(crate) static MMF_LOG_MODE: AtomicU8 = AtomicU8::new(LogMode::NoLog as u8);

/// MMF error notification emitted whenever a new error event
/// is added to MMFErr.
pub(crate) static MMF_LOG_NOTIFY: LazyLock<Notify> = LazyLock::new(Notify::new);

/// Global log event counter.
pub(crate) static MMF_LOG_INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Current timestamp format used by the framework.
pub(crate) static MMF_TIMESTAMP_FORMAT: AtomicU8 = AtomicU8::new(TimestampFormat::LocalTime as u8);

/// Timestamp format used by MMF.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimestampFormat {
    LocalTime = 0,
    UtcTime = 1,
}

/// Information about the Mesh Worker Framework.
#[derive(Debug, Clone, Copy)]
pub struct MMFInfo {
    /// crate name.
    pub crate_name: &'static str,

    /// crate version.
    pub crate_version: &'static str,

    /// Git commit used to build the crate.
    pub crate_git_commit: &'static str,

    /// Build profile ("debug" or "release").
    pub build_profile: &'static str,
}

impl std::fmt::Display for MMFInfo {
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

/// Internal MMF runtime data.
#[derive(Debug)]
pub(crate) struct MMFData {
    /// Registered worker types.
    pub(crate) worker_type_registry: WorkerTypeRegistry,
}

impl MMFData {
    pub(crate) fn new() -> Self {
        Self { worker_type_registry: WorkerTypeRegistry::new() }
    }
}

/// MMF startup configuration.
#[derive(Debug, Clone)]
pub struct MMFConfiguration {
    /// Capacity of normal worker event queues.
    pub worker_normal_channel_capacity: usize,

    /// Capacity of priority worker event queues.
    pub worker_priority_channel_capacity: usize,

    /// Timestamp format used by MMF.
    pub timestamp_format: TimestampFormat,

    /// Capacity of the MMFErr circular buffer.
    pub error_event_buffer_capacity: usize,

    /// Capacity of the MMFLog circular buffer.
    pub log_event_buffer_capacity: usize,

    /// Log file path.
    pub log_file_path: PathBuf,

    /// Maximum log file size in bytes.
    pub log_file_max_size_bytes: u64,
}

impl Default for MMFConfiguration {
    fn default() -> Self {
        let mut log_file_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("mmf.exe"));
        log_file_path.set_file_name("mmf.log");

        let log_file_max_size_bytes = 10 * 1024 * 1024;

        Self { worker_normal_channel_capacity: 50, worker_priority_channel_capacity: 10, timestamp_format: LocalTime, error_event_buffer_capacity: 100, log_event_buffer_capacity: 100, log_file_path, log_file_max_size_bytes }
    }
}

/// Public MMF entry point.
pub struct MMF;

impl MMF {
    /// Returns information about the Mesh Worker Framework.
    pub fn get_info() -> MMFInfo {
        MMFInfo { crate_name: env!("CARGO_PKG_NAME"), crate_version: env!("CARGO_PKG_VERSION"), crate_git_commit: env!("MMF_GIT_COMMIT"), build_profile: if cfg!(debug_assertions) { "debug" } else { "release" } }
    }

    /// Initialize MMF using the default configuration.
    pub fn init() -> Result<(), MMFError> {
        Self::init_with_configuration(MMFConfiguration::default())
    }

    /// Initialize the MMF framework with custom configuration.
    pub fn init_with_configuration(mut mmf_configuration: MMFConfiguration) -> Result<(), MMFError> {
        let mut mmf_data_guard = MMF_DATA.write().expect(MMFPanic::MMFData_WriteLockPoisoned.message());
        if mmf_data_guard.is_some() {
            return Err(MMFError::Framework_AlreadyInitialized);
        }
        *mmf_data_guard = Some(MMFData::new());
        drop(mmf_data_guard);

        MMF_WRK_MESSAGE_INSTANCE_COUNTER.store(1, Ordering::Relaxed);
        MMF_ERR_INSTANCE_COUNTER.store(1, Ordering::Relaxed);
        MMF_LOG_INSTANCE_COUNTER.store(1, Ordering::Relaxed);
        MMF_TIMESTAMP_FORMAT.store(mmf_configuration.timestamp_format as u8, Ordering::Relaxed);

        let mut mmf_mod_data_guard = MMF_WRK_DATA.write().expect(MMFPanic::MMFWrkData_WriteLockPoisoned.message());
        *mmf_mod_data_guard = Some(WorkerData::new(&mmf_configuration));
        drop(mmf_mod_data_guard);

        let mut mmf_err_data_guard = MMF_ERR_DATA.write().expect(MMFPanic::MMFErrData_WriteLockPoisoned.message());
        *mmf_err_data_guard = Some(ErrorData { error_event_buffer_capacity: mmf_configuration.error_event_buffer_capacity, error_event_list: VecDeque::new() });
        drop(mmf_err_data_guard);

        // Initialize the MMF log file.
        let log_file = log_file::init_log_file(&mut mmf_configuration)?;
        let mut mmf_log_data_guard = MMF_LOG_DATA.write().expect(MMFPanic::MMFLogData_WriteLockPoisoned.message());
        *mmf_log_data_guard = Some(LoggingData {
            log_event_buffer_capacity: mmf_configuration.log_event_buffer_capacity,
            logged_worker_list: Vec::new(),
            log_event_list: VecDeque::new(),
            log_file_enabled: false,
            log_file_path: mmf_configuration.log_file_path,
            log_file: Some(log_file),
            log_file_max_size_bytes: mmf_configuration.log_file_max_size_bytes,
            log_file_current_size_bytes: 0,
        });
        drop(mmf_log_data_guard);

        Ok(())
    }

    /// Shutdown the MMF framework and release all resources.
    ///
    /// All workers must be closed before calling this function.
    ///
    /// If one or more workers are still running,
    /// FrameworkError::worker_StillRunning is returned.
    ///
    /// Use MMFWrk::close_worker() to terminate workers
    /// gracefully, or MMFWrk::abort_worker() as a last
    /// resort if a worker becomes unresponsive.
    pub fn close() -> Result<(), MMFError> {
        // Check that MMF has been initialized.
        let mmf_data_guard = MMF_DATA.read().expect(MMFPanic::MMFData_ReadLockPoisoned.message());
        if mmf_data_guard.is_none() {
            return Err(MMFError::Framework_NotInitialized);
        }
        drop(mmf_data_guard);

        // Make sure no worker is still running.
        {
            let mmf_mod_data_guard = MMF_WRK_DATA.read().expect(MMFPanic::MMFWrkData_ReadLockPoisoned.message());
            let mmf_mod_data = mmf_mod_data_guard.as_ref().expect(MMFPanic::MMFWrkData_Missing.message());
            if !mmf_mod_data.worker_registry.is_empty() {
                return Err(MMFError::Framework_CannotClose_workersStillRunning);
            }
        }

        // Destroy MMF data.
        let mut mmf_data_guard = MMF_DATA.write().expect(MMFPanic::MMFData_WriteLockPoisoned.message());
        *mmf_data_guard = None;

        let mut mmf_mod_data_guard = MMF_WRK_DATA.write().expect(MMFPanic::MMFWrkData_WriteLockPoisoned.message());
        *mmf_mod_data_guard = None;

        let mut mmf_err_data_guard = MMF_ERR_DATA.write().expect(MMFPanic::MMFErrData_WriteLockPoisoned.message());
        *mmf_err_data_guard = None;

        // Destroy MMF logging data.
        //
        // Dropping LoggingData also closes the MMF log file.
        let mut mmf_log_data_guard = MMF_LOG_DATA.write().expect(MMFPanic::MMFLogData_WriteLockPoisoned.message());
        *mmf_log_data_guard = None;

        Ok(())
    }

    /// Check whether the framework is initialized.
    pub fn is_initialized() -> bool {
        MMF_WRK_DATA.read().expect(MMFPanic::MMFWrkData_ReadLockPoisoned.message()).is_some()
    }

    /// Register a worker type.
    ///
    /// worker types must be registered before they can be
    /// instantiated by MMF.
    ///
    /// Registration associates the worker's WorkerTypeID
    /// with:
    /// - its Rust type name, used for diagnostics;
    /// - a constructor function used by MMF to create new
    ///   worker instances dynamically.
    ///
    /// The registered constructor creates a new default
    /// instance of the worker each time MMF spawns it.
    ///
    /// Returns:
    /// - Framework_NotInitialized if MMF has not been initialized.
    /// - workerType_AlreadyRegistered if the worker type has
    ///   already been registered.
    /// - worker_DuplicateWorkerTypeID if the WorkerTypeID is
    ///   already associated with a different Rust worker type.
    pub fn register_worker_type<T>() -> Result<(), MMFError>
    where
        T: Worker + Send + 'static,
    {
        let mut mmf_data_guard = MMF_DATA.write().expect(MMFPanic::MMFData_WriteLockPoisoned.message());
        let mmf_data = mmf_data_guard.as_mut().ok_or(MMFError::Framework_NotInitialized)?;
        let rust_type_name = std::any::type_name::<T>();

        match mmf_data.worker_type_registry.get_worker_rust_type_name(T::worker_type_id()) {
            Ok(registered_rust_type_name) => {
                if registered_rust_type_name == rust_type_name {
                    return Err(MMFError::WorkerType_AlreadyRegistered);
                }

                return Err(MMFError::WorkerType_DuplicateWorkerTypeID);
            }

            Err(MMFError::WorkerType_NotFound) => {
                // Create the constructor associated with this worker type.
                //
                // The constructor is stored in the WorkerTypeRegistry and
                // invoked by MMF whenever a new instance of this worker
                // type must be spawned.
                //
                // Note that T::new() is not executed here.
                // Instead, a function calling T::new() is created and
                // stored for later use.
                let worker_constructor: WorkerConstructor = || Box::new(T::new());
                mmf_data.worker_type_registry.add_worker_type(T::worker_type_id(), rust_type_name, worker_constructor);
            }

            Err(error) => return Err(error),
        }

        Ok(())
    }

    /// Returns the current timestamp formatted as RFC3339.
    pub(crate) fn get_timestamp_rfc3339() -> String {
        match MMF_TIMESTAMP_FORMAT.load(Ordering::Relaxed) {
            x if x == TimestampFormat::LocalTime as u8 => Local::now().to_rfc3339_opts(SecondsFormat::Millis, false),
            x if x == TimestampFormat::UtcTime as u8 => Utc::now().to_rfc3339_opts(SecondsFormat::Millis, false),
            _ => Local::now().to_rfc3339_opts(SecondsFormat::Millis, false),
        }
    }
}
