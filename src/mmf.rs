//! Public MMF API.
//!
//! This API exposes the framework entry points used to
//! initialize and shutdown the API global runtime.

use std::sync::{LazyLock, RwLock};
use std::sync::atomic::AtomicU64;
use tokio::sync::Notify;

use crate::error::{FrameworkError, MMFPanic};
use crate::logging::LoggingData;
use crate::registry::Registry;

/// Global MMF runtime storage.
pub(crate) static MMF_DATA: LazyLock<RwLock<Option<MMFData>>> = LazyLock::new(|| RwLock::new(None));

/// Global MMF notifier used to signal child module initialized.
pub(crate) static MMF_MODULE_INIT_NOTIFY: LazyLock<Notify> = LazyLock::new(|| Notify::new());

/// Global MMF notifier used to signal child module closures.
pub(crate) static MMF_MODULE_CLOSE_NOTIFY: LazyLock<Notify> = LazyLock::new(|| Notify::new());

/// Global MMF message instance counter
pub(crate) static MESSAGE_INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(1);


/// Internal MMF runtime data.
pub(crate) struct MMFData {

    /// Registered modules and hierarchy information.
    pub(crate) registry: Registry,

    /// Logging configuration.
    pub(crate) logging_data: LoggingData,

    /// Capacity of module event queues.
    pub(crate) module_channel_capacity: usize,
}

impl MMFData {

    /// Create a new MMF runtime instance.
    fn new() -> Self {
        
        Self {
            registry: Registry::new(),
            logging_data: LoggingData::new(),
            module_channel_capacity: 500,
        }
    }

}


/// Public MMF entry point.
pub struct MMF;

impl MMF {

    /// Initialize the MMF framework.
    pub fn init() -> Result<(), FrameworkError> {
        
        {
            let mut mmf_data_guard = MMF_DATA.write().expect(MMFPanic::MMFData_WriteLockPoisoned.message());
            if mmf_data_guard.is_some() {
                return Err(FrameworkError::Framework_AlreadyInitialized);
            }
            *mmf_data_guard = Some(MMFData::new());
            drop(mmf_data_guard);
        }

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

        let mut mmf_data_guard = MMF_DATA.write().expect(MMFPanic::MMFData_WriteLockPoisoned.message());

        let mmf_data = match mmf_data_guard.as_ref() {
            Some(mmf_data) => mmf_data,
            None => return Err(FrameworkError::Framework_NotInitialized),
        };

        if !mmf_data.registry.is_empty() {
            return Err(FrameworkError::Framework_CannotClose_ModulesStillRunning);
        }

        *mmf_data_guard = None;
        drop(mmf_data_guard);

        Ok(())
    }


    /// Check whether the framework is initialized.
    pub fn is_initialized() -> bool {
        
        MMF_DATA.read().expect(MMFPanic::MMFData_ReadLockPoisoned.message()).is_some()
    }


}

