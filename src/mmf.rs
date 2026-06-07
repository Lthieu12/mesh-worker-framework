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
        drop(mmf_data_guard);

        Ok(module_id)
    }

    /// Get the current state of a module.
    pub fn get_module_state(module_id: &ModuleID) -> Result<ModuleState, FrameworkError>{

        let mmf_data_guard = MMF_DATA.read().unwrap();
        let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;
        let module_state = mmf_data.registry.get_module_state(module_id)?;
        drop(mmf_data_guard);

        Ok(module_state)
    }

    /// Initialize a module and wait until on_init() completes.
    pub async fn init_module(module_id: &ModuleID) -> Result<(), FrameworkError> {

        // Get the current module state.
        //
        // Only modules in the Created state can be initialized.
        let module_state = MMF::get_module_state(module_id)?;
        if module_state != ModuleState::Created {
            return Err(FrameworkError::Module_AlreadyInitialized);
        }

        // Change Module state to Initializing.
        // Read MMF data to retrieve the Lifecycle channel associated
        // with the target module.
        let mut mmf_data_guard = MMF_DATA.write().unwrap();
        let mmf_data = mmf_data_guard.as_mut().ok_or(FrameworkError::Framework_NotInitialized)?;
        mmf_data.registry.set_module_state(module_id, ModuleState::Initializing)?;
        let lifecycle_channel = mmf_data.registry.get_lifecycle_channel(module_id)?;
        drop(mmf_data_guard);

        // Create a temporary reply channel.
        //
        // reply_sender is sent to ModuleTask.
        // reply_receiver stays in MMF.
        //
        // MMF will wait on reply_receiver until ModuleTask
        // confirms that on_init() has completed.
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel();

        // Send the Init command to ModuleTask.
        //
        // The reply_sender is attached to the command so that
        // ModuleTask can notify MMF when on_init() completes.
        lifecycle_channel.send(LifecycleCommand::Init(reply_sender)).await.map_err(|_| FrameworkError::Module_ChannelClosed)?;

        // Wait for ModuleTask to acknowledge completion of on_init().
        //
        // This turns Init into a synchronous MMF operation:
        // init_module() returns only after the module has finished
        // executing on_init().
        reply_receiver.await.map_err(|_| FrameworkError::Module_ChannelClosed)?;

        // Re-acquire write access to MMF and update the module state.
        //
        // At this point on_init() has completed successfully.
        let mut mmf_data_guard = MMF_DATA.write().unwrap();
        let mmf_data = mmf_data_guard.as_mut().ok_or(FrameworkError::Framework_NotInitialized)?;
        mmf_data.registry.set_module_state(module_id, ModuleState::Initialized)?;
        drop(mmf_data_guard);

        Ok(())
    }

    /// Close a module and wait until on_close() completes.
    pub async fn close_module(module_id: &ModuleID) -> Result<(), FrameworkError> {

        // Get the current module state.
        //
        // Only modules in the Initialized state can be closed.
        let module_state = MMF::get_module_state(module_id)?;
        if module_state != ModuleState::Initialized {
            return Err(FrameworkError::Module_NotInitialized);
        }

        // Change Module state to Closing
        // Read MMF data to retrieve the Lifecycle channel associated
        // with the target module.
        let mut mmf_data_guard = MMF_DATA.write().unwrap();
        let mmf_data = mmf_data_guard.as_mut().ok_or(FrameworkError::Framework_NotInitialized)?;
        mmf_data.registry.set_module_state(module_id, ModuleState::Closing)?;
        let lifecycle_channel = mmf_data.registry.get_lifecycle_channel(module_id)?;
        drop(mmf_data_guard);

        // Create a temporary reply channel.
        //
        // reply_sender is sent to ModuleTask.
        // reply_receiver stays in MMF.
        //
        // MMF will wait on reply_receiver until ModuleTask
        // confirms that on_close() has completed.
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel();        

        // Send the Close command to ModuleTask.
        //
        // The reply_sender is attached to the command so that
        // ModuleTask can notify MMF when on_close() completes.
        lifecycle_channel.send(LifecycleCommand::Close(reply_sender)).await.map_err(|_| FrameworkError::Module_ChannelClosed)?;

        // Wait for ModuleTask to acknowledge completion of on_close().
        //
        // This turns Close into a synchronous MMF operation:
        // close_module() returns only after the module has finished
        // executing on_close().
        reply_receiver.await.map_err(|_| FrameworkError::Module_ChannelClosed)?;

        // Read MMF data to take ownership of the JoinHandle associated
        // with the target module.
        //
        // The JoinHandle is removed from the Registry because it will
        // be consumed by await.
        let mut mmf_data_guard = MMF_DATA.write().unwrap();
        let mmf_data = mmf_data_guard.as_mut().ok_or(FrameworkError::Framework_NotInitialized)?;
        let join_handle = mmf_data.registry.take_join_handle(module_id)?;
        drop(mmf_data_guard);

        // Wait until the Tokio task associated with the module
        // has fully terminated.
        //
        // At this point on_close() has already completed and the
        // ModuleTask should exit its event loop.
        join_handle.await.map_err(|_| FrameworkError::Framework_InternalError)?;

        // Mark the module as Closed.
        //
        // Closed indicates that the module lifecycle has completed
        // successfully.
        let mut mmf_data_guard = MMF_DATA.write().unwrap();
        let mmf_data = mmf_data_guard.as_mut().ok_or(FrameworkError::Framework_NotInitialized)?;
        mmf_data.registry.set_module_state(module_id, ModuleState::Closed)?;
        drop(mmf_data_guard);

        // Remove the module from the Registry.
        //
        // Closed and removal are intentionally treated as separate
        // operations. Future implementations may keep Closed modules
        // available for inspection or restart before removal.
        let mut mmf_data_guard = MMF_DATA.write().unwrap();
        let mmf_data = mmf_data_guard.as_mut().ok_or(FrameworkError::Framework_NotInitialized)?;
        mmf_data.registry.remove_module(module_id)?;
        drop(mmf_data_guard);

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