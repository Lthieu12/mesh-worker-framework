//! Public MMF API.
//!
//! This module exposes the framework entry points used to
//! initialize, shutdown and manage modules.

use std::sync::{LazyLock, RwLock};

use crate::error::FrameworkError;
use crate::logging::LoggingData;
use crate::module::{Module, ModuleID, ModuleState};
use crate::registry::Registry;
use crate::module_task::spawn_module_task;
use crate::channel::LifecycleCommand;

/// Global MMF runtime storage.
static MMF_DATA: LazyLock<RwLock<Option<MMFData>>> = LazyLock::new(|| RwLock::new(None));

/// Internal MMF runtime data.
struct MMFData {

    /// Registered modules and hierarchy information.
    registry: Registry,

    /// Logging configuration.
    logging_data: LoggingData,

    /// Capacity of module event queues.
    module_channel_capacity: usize,
}

impl MMFData {

    /// Create a new MMF runtime instance.
    fn new() -> Self {
        Self {
            registry: Registry::new(),
            logging_data: LoggingData::new(),
            module_channel_capacity: 100,
        }
    }
}

/// Public MMF entry point.
pub struct MMF;

impl MMF {

    /// Initialize the MMF framework.
    pub fn init() -> Result<(), FrameworkError> {
        {
            let mut mmf_data = MMF_DATA.write().unwrap();

            if mmf_data.is_some() {
                return Err(FrameworkError::Framework_AlreadyInitialized);
            }

            *mmf_data = Some(MMFData::new());
        }

        Ok(())
    }

    /// Shutdown the MMF framework and release all resources.
    pub fn close() -> Result<(), FrameworkError> {
        {
            let mut mmf_data = MMF_DATA.write().unwrap();

            if mmf_data.is_none() {
                return Err(FrameworkError::Framework_NotInitialized);
            }

            *mmf_data = None;
        }

        Ok(())
    }

    /// Check whether the framework is initialized.
    pub(crate) fn is_initialized() -> bool {
        MMF_DATA.read().unwrap().is_some()
    }

    /// Create and register a new module instance.
    ///
    /// Steps:
    /// 1. Generate a unique ModuleID.
    /// 2. Create communication channels used by MMF.
    /// 3. Spawn the Tokio task that will own the module instance.
    /// 4. Register the module resources in the Registry.
    ///
    /// After this function returns:
    /// - MMF owns the ModuleID, channels and JoinHandle.
    /// - The Tokio task owns the module instance itself.
    pub fn spawn_module<T>(module: T, father_module_id: Option<ModuleID>) -> Result<ModuleID, FrameworkError>
    where T: Module + Send + 'static {

        let mut mmf_data_guard = MMF_DATA.write().unwrap();

        let mmf_data = mmf_data_guard.as_mut().ok_or(FrameworkError::Framework_NotInitialized)?;

        let module_id = mmf_data.registry.generate_module_id::<T>();

        // Channel used to send Lifecycle commands (like Init and Close) to the module.
        let (lifecycle_sender, lifecycle_receiver) = tokio::sync::mpsc::channel(mmf_data.module_channel_capacity);

        // Channel used to send FrameworkMessages to the module.
        let (message_sender, message_receiver) = tokio::sync::mpsc::channel(mmf_data.module_channel_capacity);

        // Spawn the Tokio task that will own the module instance.
        //
        // Once spawned, MMF no longer owns the module.
        // MMF communicates with the module through channels and
        // tracks its execution through the JoinHandle.
        let join_handle = spawn_module_task(module, lifecycle_receiver, message_receiver);

        // Store the module resources in the Registry.
        //
        // The Registry stores:
        // - Module hierarchy information.
        // - Module state.
        // - Communication channels.
        // - JoinHandle associated with the Tokio task.
        mmf_data.registry.add_module(module_id.clone(), father_module_id, lifecycle_sender, message_sender, join_handle,)?;

        Ok(module_id)
    }

    pub fn get_module_state(module_id: &ModuleID) -> Result<ModuleState, FrameworkError>{

        let mmf_data_guard = MMF_DATA.read().unwrap();

        let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;

        let module_state = mmf_data.registry.get_module_state(module_id)?;

        Ok(module_state)
    }

    pub async fn init_module(module_id: &ModuleID) -> Result<(), FrameworkError> {

        let module_state = MMF::get_module_state(module_id)?;

        if module_state != ModuleState::Created {

            return Err(FrameworkError::Module_AlreadyInitialized);
        }

        let mmf_data_guard = MMF_DATA.read().unwrap();

        let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;

        let lifecycle_channel = mmf_data.registry.get_lifecycle_channel(module_id)?;

        drop(mmf_data_guard);

        lifecycle_channel.send(LifecycleCommand::Init).await.map_err(|_| FrameworkError::Module_ChannelClosed)?;

        let mut mmf_data_guard = MMF_DATA.write().unwrap();

        let mmf_data = mmf_data_guard.as_mut().ok_or(FrameworkError::Framework_NotInitialized)?;

        mmf_data.registry.set_module_state(module_id, ModuleState::Initialized)?;

        Ok(())
    }

}


#[cfg(test)]
mod tests {

    use crate::error::FrameworkError;
    use crate::mmf::MMF;

    #[test]
    fn test_mmf_init_close() {
        assert!(!MMF::is_initialized());

        assert!(MMF::init().is_ok());

        assert!(MMF::is_initialized());

        assert!(MMF::close().is_ok());

        assert!(!MMF::is_initialized());
    }

    #[test]
    fn test_mmf_double_init() {
        assert!(MMF::init().is_ok());

        assert!(
            matches!(
                MMF::init(),
                Err(FrameworkError::Framework_AlreadyInitialized)
            )
        );

        assert!(MMF::close().is_ok());
    }

    #[test]
    fn test_mmf_double_close() {
        assert!(
            matches!(
                MMF::close(),
                Err(FrameworkError::Framework_NotInitialized)
            )
        );
    }
}