//! Public MMF API.
//!
//! This API exposes the framework entry points used to
//! initialize and shutdown the API global runtime.

use std::sync::{LazyLock, RwLock};
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::collections::VecDeque;
use tokio::sync::Notify;

use crate::framework_error::{FrameworkError, MMFPanic};
use crate::module_type_registry::{ModuleTypeRegistry};
use crate::module::{Module, ModuleConstructor};
use crate::mmf_mod::{ModuleData};
use crate::mmf_err::{ErrorData};
use crate::mmf_log::{LogMode, LoggingData};


/// Global MMF runtime storage shared by all
/// framework components.
pub(crate) static MMF_DATA: LazyLock<RwLock<Option<MMFData>>> = LazyLock::new(|| RwLock::new(None));


/// Global MMF module runtime storage.
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



/// Internal MMF runtime data.
#[derive(Debug)]
pub(crate) struct MMFData {

    /// Registered module types.
    pub(crate) module_type_registry: ModuleTypeRegistry,
}

impl MMFData {

    pub(crate) fn new() -> Self {

        Self {
            module_type_registry: ModuleTypeRegistry::new(),
        }
    }
}


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
        
        let mut mmf_data_guard = MMF_DATA.write().expect(MMFPanic::MMFData_WriteLockPoisoned.message());
        if mmf_data_guard.is_some() {
            return Err(FrameworkError::Framework_AlreadyInitialized);
        }
        *mmf_data_guard = Some(MMFData::new());
        drop(mmf_data_guard);

        MMF_MOD_MESSAGE_INSTANCE_COUNTER.store(1, Ordering::Relaxed);
        MMF_ERR_INSTANCE_COUNTER.store(1, Ordering::Relaxed);
        MMF_LOG_INSTANCE_COUNTER.store(1, Ordering::Relaxed);

        let mut mmf_mod_data_guard = MMF_MOD_DATA.write().expect(MMFPanic::MMFModData_WriteLockPoisoned.message());
        *mmf_mod_data_guard = Some(ModuleData::new(&mmf_configuration));
        drop(mmf_mod_data_guard);

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

        // Check that MMF has been initialized.
        let mmf_data_guard = MMF_DATA.read().expect(MMFPanic::MMFData_ReadLockPoisoned.message());
        if mmf_data_guard.is_none() {
            return Err(FrameworkError::Framework_NotInitialized);
        }
        drop(mmf_data_guard);

        // Make sure no module is still running.
        {
            let mmf_mod_data_guard = MMF_MOD_DATA.read().expect(MMFPanic::MMFModData_ReadLockPoisoned.message());
            let mmf_mod_data = mmf_mod_data_guard.as_ref().expect(MMFPanic::MMFModData_Missing.message());
            if !mmf_mod_data.module_registry.is_empty() {
                return Err(FrameworkError::Framework_CannotClose_ModulesStillRunning);
            }
        }

        // Destroy MMF data.
        let mut mmf_data_guard = MMF_DATA.write().expect(MMFPanic::MMFData_WriteLockPoisoned.message());
        *mmf_data_guard = None;

        let mut mmf_mod_data_guard = MMF_MOD_DATA.write().expect(MMFPanic::MMFModData_WriteLockPoisoned.message());
        *mmf_mod_data_guard = None;

        let mut mmf_err_data_guard = MMF_ERR_DATA.write().expect(MMFPanic::MMFErrData_WriteLockPoisoned.message());
        *mmf_err_data_guard = None;

        let mut mmf_log_data_guard = MMF_LOG_DATA.write().expect(MMFPanic::MMFLogData_WriteLockPoisoned.message());
        *mmf_log_data_guard = None;

        Ok(())
    }


    /// Check whether the framework is initialized.
    pub fn is_initialized() -> bool {
        
        MMF_MOD_DATA.read().expect(MMFPanic::MMFModData_ReadLockPoisoned.message()).is_some()
    }


    /// Register a module type.
    ///
    /// Module types must be registered before they can be
    /// instantiated by MMF.
    ///
    /// Registration associates the module's ModuleTypeID
    /// with:
    /// - its Rust type name, used for diagnostics;
    /// - a constructor function used by MMF to create new 
    ///   module instances dynamically.
    ///
    /// The registered constructor creates a new default
    /// instance of the module each time MMF spawns it.
    ///
    /// Returns:
    /// - Framework_NotInitialized if MMF has not been initialized.
    /// - ModuleType_AlreadyRegistered if the module type has
    ///   already been registered.
    /// - Module_DuplicateModuleTypeID if the ModuleTypeID is
    ///   already associated with a different Rust module type.
    pub fn register_module_type<T>() -> Result<(), FrameworkError>
    where T: Module + Send + 'static {

        let mut mmf_data_guard = MMF_DATA.write().expect(MMFPanic::MMFData_WriteLockPoisoned.message());
        let mmf_data = mmf_data_guard.as_mut().ok_or(FrameworkError::Framework_NotInitialized)?;
        let rust_type_name = std::any::type_name::<T>();
        
        match mmf_data.module_type_registry.get_module_rust_type_name(T::module_type_id())
        {
            Ok(registered_rust_type_name) => {

                if registered_rust_type_name == rust_type_name {
                    return Err(FrameworkError::ModuleType_AlreadyRegistered);
                }

                return Err(FrameworkError::ModuleType_DuplicateModuleTypeID);
            }

            Err(FrameworkError::ModuleType_NotFound) => {

                // Create the constructor associated with this module type.
                //
                // The constructor is stored in the ModuleTypeRegistry and
                // invoked by MMF whenever a new instance of this module
                // type must be spawned.
                //
                // Note that T::new() is not executed here.
                // Instead, a function calling T::new() is created and
                // stored for later use.
                let module_constructor: ModuleConstructor = || Box::new(T::new());
                mmf_data.module_type_registry.add_module_type(T::module_type_id(), rust_type_name, module_constructor);
            }

            Err(error) => return Err(error),
        }

        Ok(())
    }

}

