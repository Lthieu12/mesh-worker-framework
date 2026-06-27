//! Public MMF API.
//!
//! This API exposes the framework entry points used to
//! initialize and shutdown the API global runtime.

use std::sync::{LazyLock, RwLock};
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::collections::VecDeque;
use tokio::sync::Notify;

use crate::framework_error::{FrameworkError, MMFPanic};
use crate::mmf_mod::{ModuleData};
use crate::mmf_err::{ErrorData};
use crate::mmf_log::{LogMode, LoggingData};


/// Global MMF runtime storage.
pub(crate) static MMF_MOD_DATA: LazyLock<RwLock<Option<ModuleData>>> = LazyLock::new(|| RwLock::new(None));

/// Global MMF notifier used to signal child module initialized.
pub(crate) static MMF_MOD_INIT_NOTIFY: LazyLock<Notify> = LazyLock::new(|| Notify::new());

/// Global MMF notifier used to signal child module closures.
pub(crate) static MMF_MOD_CLOSE_NOTIFY: LazyLock<Notify> = LazyLock::new(|| Notify::new());

/// Global MMF message instance counter used to identify messages sent to modules.
pub(crate) static MMF_MOD_MESSAGE_INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(1);


/// Global MMF error event storage.
pub(crate) static MMF_ERR_DATA: LazyLock<RwLock<Option<ErrorData>>> = LazyLock::new(|| RwLock::new(None));

/// MMF error notification emitted whenever a new error event
/// is added to MMFErr.
pub(crate) static MMF_ERR_NOTIFY: LazyLock<Notify> = LazyLock::new(|| Notify::new());

/// Global MMF error instance counter.
pub(crate) static MMF_ERR_INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(1);


/// Global logging storage.
pub(crate) static MMF_LOG_DATA: LazyLock<RwLock<Option<LoggingData>>> = LazyLock::new(|| RwLock::new(None));

/// Current logging mode.
pub(crate) static MMF_LOG_MODE: AtomicU8 = AtomicU8::new(LogMode::NoLog as u8);

/// MMF error notification emitted whenever a new error event
/// is added to MMFErr.
pub(crate) static MMF_LOG_NOTIFY: LazyLock<Notify> = LazyLock::new(|| Notify::new());

/// Global log event counter.
pub(crate) static MMF_LOG_INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(1);


/// MMF startup configuration.
#[derive(Debug, Clone)]
pub struct MMFConfiguration {

    /// Capacity of normal module event queues.
    pub module_normal_channel_capacity: usize,

    /// Capacity of priority module event queues.
    pub module_priority_channel_capacity: usize,

    /// Capacity of the MMFErr circular buffer.
    pub error_event_buffer_capacity: usize,

    /// Capacity of the MMFLog circular buffer.
    pub log_event_buffer_capacity: usize,
}

impl Default for MMFConfiguration {

    fn default() -> Self {

        Self {

            module_normal_channel_capacity: 50,
            module_priority_channel_capacity: 10,
            error_event_buffer_capacity: 100,
            log_event_buffer_capacity: 100,
        }
    }
}




/// Public MMF entry point.
pub struct MMF;

impl MMF {

    /// Initialize MMF using the default configuration.
    pub fn init() -> Result<(), FrameworkError> {

        Self::init_with_configuration(MMFConfiguration::default())
    }

    /// Initialize the MMF framework with custom configuration.
    pub fn init_with_configuration(mmf_configuration: MMFConfiguration) -> Result<(), FrameworkError> {
        
        MMF_MOD_MESSAGE_INSTANCE_COUNTER.store(1, Ordering::Relaxed);
        MMF_ERR_INSTANCE_COUNTER.store(1, Ordering::Relaxed);
        MMF_LOG_INSTANCE_COUNTER.store(1, Ordering::Relaxed);

        let mut mmf_data_guard = MMF_MOD_DATA.write().expect(MMFPanic::MMFData_WriteLockPoisoned.message());
        if mmf_data_guard.is_some() {
            return Err(FrameworkError::Framework_AlreadyInitialized);
        }
        *mmf_data_guard = Some(ModuleData::new(&mmf_configuration));
        drop(mmf_data_guard);

        let mut mmf_err_data_guard = MMF_ERR_DATA.write().expect(MMFPanic::MMFErrData_WriteLockPoisoned.message());
        *mmf_err_data_guard = Some(ErrorData {
            
            error_event_buffer_capacity:mmf_configuration.error_event_buffer_capacity,
            error_event_list: VecDeque::new(),
        });
        drop(mmf_err_data_guard);

        let mut mmf_log_data_guard = MMF_LOG_DATA.write().expect(MMFPanic::MMFLogData_WriteLockPoisoned.message());
        *mmf_log_data_guard = Some(LoggingData {
            
            log_event_buffer_capacity:mmf_configuration.log_event_buffer_capacity,
            logged_module_list: Vec::new(),
            log_event_list: VecDeque::new(),
        });
        drop(mmf_log_data_guard);

        Ok(())
    }


    /// Shutdown the MMF framework and release all resources.
    ///
    /// All modules must be closed before calling this function.
    ///
    /// If one or more modules are still running,
    /// FrameworkError::Module_StillRunning is returned.
    ///
    /// Use MMFMod::close_module() to terminate modules
    /// gracefully, or MMFMod::abort_module() as a last
    /// resort if a module becomes unresponsive.
    pub fn close() -> Result<(), FrameworkError> {

        let mut mmf_data_guard = MMF_MOD_DATA.write().expect(MMFPanic::MMFData_WriteLockPoisoned.message());

        let mmf_data = match mmf_data_guard.as_ref() {
            Some(mmf_data) => mmf_data,
            None => return Err(FrameworkError::Framework_NotInitialized),
        };

        if !mmf_data.registry.is_empty() {
            return Err(FrameworkError::Framework_CannotClose_ModulesStillRunning);
        }

        *mmf_data_guard = None;
        drop(mmf_data_guard);

        let mut mmf_err_data_guard = MMF_ERR_DATA.write().expect(MMFPanic::MMFErrData_WriteLockPoisoned.message());
        *mmf_err_data_guard = None;
        drop(mmf_err_data_guard);

        let mut mmf_log_data_guard = MMF_LOG_DATA.write().expect(MMFPanic::MMFLogData_WriteLockPoisoned.message());
        *mmf_log_data_guard = None;
        drop(mmf_log_data_guard);

        Ok(())
    }


    /// Check whether the framework is initialized.
    pub fn is_initialized() -> bool {
        
        MMF_MOD_DATA.read().expect(MMFPanic::MMFData_ReadLockPoisoned.message()).is_some()
    }


}

