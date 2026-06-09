//! Public MMF API.
//!
//! This module exposes the framework entry points used to
//! initialize, shutdown and manage modules.

use std::sync::{LazyLock, RwLock};
use bytes::Bytes;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::FrameworkError;
use crate::logging::LoggingData;
use crate::module::{Module, ModuleID, EntityID, ModuleState};
use crate::registry::Registry;
use crate::module_task::spawn_module_task;
use crate::channel::ModuleCommand;
use crate::message::{FrameworkMessage, MessageID, MessagePriority, MessageInstanceID};

/// Global MMF runtime storage.
static MMF_DATA: LazyLock<RwLock<Option<MMFData>>> = LazyLock::new(|| RwLock::new(None));

/// Global MMF message instance counter
static MESSAGE_INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(1);

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
            let mut mmf_data_guard = MMF_DATA.write().unwrap();
            if mmf_data_guard.is_some() {
                return Err(FrameworkError::Framework_AlreadyInitialized);
            }
            *mmf_data_guard = Some(MMFData::new());
            drop(mmf_data_guard);
        }

        Ok(())
    }

    /// Shutdown the MMF framework and release all resources.
    pub fn close() -> Result<(), FrameworkError> {
        {
            let mut mmf_data_guard = MMF_DATA.write().unwrap();
            if mmf_data_guard.is_none() {
                return Err(FrameworkError::Framework_NotInitialized);
            }
            *mmf_data_guard = None;
            drop(mmf_data_guard);
        }

        Ok(())
    }

    /// Generate a unique MessageInstanceID.
    fn generate_message_instance_id() -> Result<MessageInstanceID, FrameworkError> {
        let current_id = MESSAGE_INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed);
        if current_id == u64::MAX {
            return Err(FrameworkError::Framework_InternalError);
        }

        Ok(current_id)
    }

    /// Check whether the framework is initialized.
    pub fn is_initialized() -> bool {
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
    pub fn spawn_module<T>(module: T, father_id: EntityID) -> Result<ModuleID, FrameworkError>
    where T: Module + Send + 'static {

        let mut mmf_data_guard = MMF_DATA.write().unwrap();
        let mmf_data = mmf_data_guard.as_mut().ok_or(FrameworkError::Framework_NotInitialized)?;
        let module_id = mmf_data.registry.generate_module_id::<T>();

        // Channel used to send Module commands (like Init and Close) and messages to the module.
        let (module_sender, module_receiver) = tokio::sync::mpsc::channel(mmf_data.module_channel_capacity);

        // Spawn the Tokio task that will own the module instance.
        //
        // Once spawned, MMF no longer owns the module.
        // MMF communicates with the module through channels and
        // tracks its execution through the JoinHandle.
        let join_handle = spawn_module_task(module, module_receiver, module_id.clone(), father_id.clone());

        // Store the module resources in the Registry.
        //
        // The Registry stores:
        // - Module hierarchy information.
        // - Module state.
        // - Communication channels.
        // - JoinHandle associated with the Tokio task.
        mmf_data.registry.add_module(module_id.clone(), father_id, module_sender, join_handle,)?;
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

        // Change Module state to Initializing
        // and read MMF data to retrieve the Module channel
        // associated with the target module.
        //
        // Only modules in the Created state can be initialized.
        let mut mmf_data_guard = MMF_DATA.write().unwrap();
        let mmf_data = mmf_data_guard.as_mut().ok_or(FrameworkError::Framework_NotInitialized)?;
        let module_state = mmf_data.registry.get_module_state(module_id)?;
        if module_state != ModuleState::Created {
            return Err(FrameworkError::Module_AlreadyInitialized);
        }
        mmf_data.registry.set_module_state(module_id, ModuleState::Initializing)?;
        let module_channel = mmf_data.registry.get_module_channel(module_id)?;
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
        module_channel.send(ModuleCommand::Init(reply_sender)).await.map_err(|_| FrameworkError::Module_ChannelClosed)?;

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

        // Change Module state to Closing
        // and read MMF data to retrieve the Module channel
        // associated with the target module.
        //
        // Only modules in the Initialized state can be closed.
        let mut mmf_data_guard = MMF_DATA.write().unwrap();
        let mmf_data = mmf_data_guard.as_mut().ok_or(FrameworkError::Framework_NotInitialized)?;
        let module_state = mmf_data.registry.get_module_state(module_id)?;
        if module_state != ModuleState::Initialized {
            return Err(FrameworkError::Module_NotInitialized);
        }
        mmf_data.registry.set_module_state(module_id, ModuleState::Closing)?;
        let module_channel = mmf_data.registry.get_module_channel(module_id)?;
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
        module_channel.send(ModuleCommand::Close(reply_sender)).await.map_err(|_| FrameworkError::Module_ChannelClosed)?;

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

    /// Get the module FatherID of a module
    pub fn get_father_id(module_id: &ModuleID) -> Result<EntityID, FrameworkError>{
        
        let mmf_data_guard = MMF_DATA.read().unwrap();
        let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;
        let father_id = mmf_data.registry.get_father_id(module_id)?;
        drop(mmf_data_guard);

        Ok(father_id)
    }

    /// Get the children ModuleIDs of a module.
    pub fn get_children_ids(module_id: &ModuleID) -> Result<Vec<ModuleID>, FrameworkError> {
        let mmf_data_guard = MMF_DATA.read().unwrap();
        let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;

        mmf_data.registry.get_children_ids(module_id)
    }

    /// Send a message to a module.
    pub async fn send_message(sender_id: &EntityID, receiver_id: &ModuleID, message_id: &MessageID, message_data: Bytes) -> Result<(), FrameworkError> {
        
        // Read MMF data.
        //
        // send_message() only requires a read lock because it does
        // not modify the Registry or ModuleState.
        let mmf_data_guard = MMF_DATA.read().unwrap();

        let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;

        // Validate the sender.
        //
        // MMF modules must be initialized before they can
        // send messages through MMF.
        //
        // External senders are not validated.
        match sender_id {
            EntityID::Module(module_id) => {
                let module_state = mmf_data.registry.get_module_state(module_id)?;
                if module_state != ModuleState::Initialized {
                    return Err(FrameworkError::Module_NotInitialized);
                }
            }
            EntityID::External(_) => {}
        }

        // Validate the receiver module state.
        //
        // Messages can only be sent to initialized modules.
        let module_state = mmf_data.registry.get_module_state(receiver_id)?;
        if module_state != ModuleState::Initialized {
            return Err(FrameworkError::Module_NotInitialized);
        }

        // Retrieve the communication channel associated with
        // the receiver module.
        //
        // State validation and channel retrieval are performed
        // under the same lock to avoid race conditions with
        // close_module().
        let module_channel = mmf_data.registry.get_module_channel(receiver_id)?;

        // Generate the unique id of the message
        let message_instance_id = MMF::generate_message_instance_id()?;

        // Create the FrameworkMessage.
        //
        // MMF is responsible for generating framework-managed
        // fields such as MessageInstanceID.
        let framework_message = FrameworkMessage {

                sender_id: sender_id.clone(),
                receiver_id: receiver_id.clone(),
                message_id: message_id.clone(),
                message_instance_id,
                message_priority: MessagePriority::Normal,
                message_data,
            };

        // Release MMF lock before awaiting.
        drop(mmf_data_guard);

        // Send the message to the target module.
        module_channel.send(ModuleCommand::Message(framework_message)).await.map_err(|_| FrameworkError::Module_ChannelClosed)?;

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