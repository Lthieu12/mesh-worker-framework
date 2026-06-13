//! Public MMF API.
//!
//! This module exposes the framework entry points used to
//! initialize, shutdown and manage modules.

use std::sync::{LazyLock, RwLock};
use bytes::Bytes;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::Notify;

use crate::error::FrameworkError;
use crate::logging::LoggingData;
use crate::module::{Module, ModuleID, EntityID, ModuleState};
use crate::registry::Registry;
use crate::module_task::spawn_module_task;
use crate::channel::ModuleCommand;
use crate::message::{FrameworkMessage, MessageID, MessageInstanceID};

/// Global MMF runtime storage.
pub(crate) static MMF_DATA: LazyLock<RwLock<Option<MMFData>>> = LazyLock::new(|| RwLock::new(None));

/// Global MMF notifier used to signal child module initialized.
pub(crate) static MMF_MODULE_INIT_NOTIFY: LazyLock<Notify> = LazyLock::new(|| Notify::new());

/// Global MMF notifier used to signal child module closures.
pub(crate) static MMF_MODULE_CLOSE_NOTIFY: LazyLock<Notify> = LazyLock::new(|| Notify::new());

/// Global MMF message instance counter
static MESSAGE_INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Internal MMF runtime data.
pub(crate) struct MMFData {

    /// Registered modules and hierarchy information.
    pub(crate) registry: Registry,

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

        let mut mmf_data_guard = MMF_DATA.write().unwrap();

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
    pub fn spawn_module<T>(module: T, father_id: &EntityID) -> Result<ModuleID, FrameworkError>
    where T: Module + Send + 'static {

        let mut mmf_data_guard = MMF_DATA.write().unwrap();
        let mmf_data = mmf_data_guard.as_mut().ok_or(FrameworkError::Framework_NotInitialized)?;
        
        // Child modules can only be attached to an
        // initialized parent module.
        //
        // This prevents creating new hierarchy branches
        // while the parent is starting up or shutting down.
        match &father_id {

            EntityID::Module(father_id_ref) => {

                let module_state = mmf_data.registry.get_module_state(father_id_ref)?;

                if module_state != ModuleState::Initialized {
                    return Err(FrameworkError::Module_NotInitialized);
                }
            }

            EntityID::External(_) => {}
        }
        
        let module_id = mmf_data.registry.generate_module_id::<T>();

        // Channel used to send Module commands (like Init and Close) and messages to the module.
        let (normal_sender_channel, normal_receiver_channel) = tokio::sync::mpsc::channel(mmf_data.module_channel_capacity);
        // Channel used to send priority Module commands (like Init and Close) and messages to the module.
        let (priority_sender_channel, priority_receiver_channel) = tokio::sync::mpsc::channel(mmf_data.module_channel_capacity);

        // Spawn the Tokio task that will own the module instance.
        //
        // Once spawned, MMF no longer owns the module.
        // MMF communicates with the module through channels and
        // tracks its execution through the JoinHandle.
        let join_handle = spawn_module_task(module, normal_receiver_channel, priority_receiver_channel, module_id.clone(), father_id.clone());

        // Store the module resources in the Registry.
        //
        // The Registry stores:
        // - Module hierarchy information.
        // - Module state.
        // - Communication channels.
        // - JoinHandle associated with the Tokio task.
        mmf_data.registry.add_module(&module_id, &father_id, normal_sender_channel, priority_sender_channel, join_handle,)?;
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


    /// Request module initialization.
    ///
    /// The module state is immediately changed to Initializing.
    /// This function only queues the Init command and does not wait
    /// for on_init() completion.
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
        let normal_sender_channel = mmf_data.registry.get_module_normal_channel(module_id)?;
        drop(mmf_data_guard);

        // Send the Init command to ModuleTask.
        normal_sender_channel.send(ModuleCommand::Init).await.map_err(|_| FrameworkError::Module_ChannelClosed)?;

        Ok(())
    }


    /// Request module shutdown.
    ///
    /// The module state is immediately changed to Closing.
    /// This function only queues the Close command and does not wait
    /// for on_close() completion.
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
        let normal_sender_channel = mmf_data.registry.get_module_normal_channel(module_id)?;
        drop(mmf_data_guard);

        // Send the Close command to ModuleTask.
        normal_sender_channel.send(ModuleCommand::Close).await.map_err(|_| FrameworkError::Module_ChannelClosed)?;

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


    /// Get the child ModuleIDs of an entity.
    pub fn get_children_ids(entity_id: &EntityID) -> Result<Vec<ModuleID>, FrameworkError> {
        
        let mmf_data_guard = MMF_DATA.read().unwrap();
        let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;

        mmf_data.registry.get_children_ids(entity_id)
    }


    /// Get the number of child modules of an entity.
    pub fn get_children_count(entity_id: &EntityID) -> Result<usize, FrameworkError> {

        let mmf_data_guard = MMF_DATA.read().unwrap();
        let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;
        let children_count = mmf_data.registry.get_children_count(entity_id)?;
        drop(mmf_data_guard);
        Ok(children_count)
    }


    /// Wait until all modules in the list reach the Initialized state.
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    pub async fn wait_module_list_init(module_ids: &Vec<ModuleID>, timeout_ms: Option<u64>) -> Result<(), FrameworkError> {

        let wait_future = async {

            loop {

                // Register interest in the next initialization event
                // before checking module states to avoid lost notifications.
                let init_notify = MMF_MODULE_INIT_NOTIFY.notified();

                // Read Registry state under a short-lived lock.
                //
                // The lock must be released before awaiting the
                // next notification.
                let all_initialized = {

                    let mmf_data_guard = MMF_DATA.read().unwrap();
                    let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;

                    // Initialization is complete only when all modules
                    // have reached the Initialized state.
                    module_ids.iter().all(|module_id| { 

                        matches!(mmf_data.registry.get_module_state(module_id), Ok(ModuleState::Initialized))
                    })
                };

                if all_initialized {
                    return Ok(());
                }

                // Wait for the next module initialization event.
                init_notify.await;
            }
        };

        match timeout_ms {

            Some(timeout_ms) => {
                tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), wait_future).await.map_err(|_| FrameworkError::Operation_Timeout)?
            }
            None => {
                wait_future.await
            }
        }
    }


    /// Wait until a module reaches the Initialized state.
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    pub async fn wait_module_init(module_id: &ModuleID, timeout_ms: Option<u64>) -> Result<(), FrameworkError> {

        MMF::wait_module_list_init(&vec![module_id.clone()], timeout_ms).await
    }


    /// Wait until all direct children of an entity reach the
    /// Initialized state.
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    pub async fn wait_children_init(entity_id: &EntityID, timeout_ms: Option<u64>) -> Result<(), FrameworkError> {

        // Resolve the current child list and delegate the wait
        // operation to wait_module_list_init().
        MMF::wait_module_list_init(&MMF::get_children_ids(entity_id)?, timeout_ms).await
    }


    /// Wait until all modules in the list have fully closed.
    ///
    /// A module is considered closed once it has been removed
    /// from the Registry by ModuleTask.
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    pub async fn wait_module_list_close(module_ids: &Vec<ModuleID>, timeout_ms: Option<u64>) -> Result<(), FrameworkError> {

        let wait_future = async {

            loop {

                // Register interest in the next close event
                // before checking module states to avoid lost notifications.
                let close_notify = MMF_MODULE_CLOSE_NOTIFY.notified();

                // Read Registry state under a short-lived lock.
                //
                // The lock must be released before awaiting the
                // next notification.
                let all_closed = {

                    let mmf_data_guard = MMF_DATA.read().unwrap();
                    let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;

                    // Shutdown is complete only when all modules
                    // have been removed from the Registry.
                    module_ids.iter().all(|module_id| {
                        !mmf_data.registry.contains_module(module_id)
                    })
                };

                if all_closed {
                    return Ok(());
                }

                // Wait for the next module shutdown event.
                close_notify.await;
            }
        };

        match timeout_ms {

            Some(timeout_ms) => {
                tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), wait_future).await.map_err(|_| FrameworkError::Operation_Timeout)?
            }
            None => {
                wait_future.await
            }
        }
    }


    /// Wait until a module has fully closed.
    ///
    /// A module is considered closed once it has been removed
    /// from the Registry by ModuleTask.
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    pub async fn wait_module_close(module_id: &ModuleID, timeout_ms: Option<u64>) -> Result<(), FrameworkError> {

        MMF::wait_module_list_close(&vec![module_id.clone()], timeout_ms).await
    }


    /// Wait until all direct children of an entity have fully closed.
    ///
    /// A child is considered closed once it has been removed from
    /// the Registry by ModuleTask.
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    pub async fn wait_children_close(entity_id: &EntityID, timeout_ms: Option<u64>) -> Result<(), FrameworkError> {

        // Resolve the current child list and delegate the wait
        // operation to wait_module_list_close().
        MMF::wait_module_list_close(&MMF::get_children_ids(entity_id)?, timeout_ms).await
    }


    /// Forcefully terminate a list of modules.
    ///
    /// This operation aborts the Tokio tasks associated with the
    /// modules and removes them from the Registry without executing
    /// any additional module code.
    ///
    /// This function should only be used as a recovery mechanism
    /// when normal shutdown has failed.
    pub fn kill_module_list(module_ids: &Vec<ModuleID>) -> Result<(), FrameworkError> {

        let mut mmf_data_guard = MMF_DATA.write().unwrap();
        let mmf_data = mmf_data_guard.as_mut().ok_or(FrameworkError::Framework_NotInitialized)?;

        for module_id in module_ids {

            let join_handle = mmf_data.registry.take_join_handle(module_id)?;

            // Abort the Tokio task immediately.
            join_handle.abort();

            // Remove the module from the Registry.
            mmf_data.registry.remove_module(module_id)?;
        }

        drop(mmf_data_guard);

        // Wake up all waiters interested in module
        // shutdown events.
        MMF_MODULE_CLOSE_NOTIFY.notify_waiters();

        Ok(())
    }

    
    /// Forcefully terminate a module.
    ///
    /// This operation aborts the Tokio tasks associated with the
    /// module and removes them from the Registry without executing
    /// any additional module code.
    ///
    /// This function should only be used as a recovery mechanism
    /// when normal shutdown has failed.
    pub fn kill_module(module_id: &ModuleID) -> Result<(), FrameworkError> {

        MMF::kill_module_list(&vec![module_id.clone()])
    }


    /// Forcefully terminate  all direct children modules of an entity.
    ///
    /// This operation aborts the Tokio tasks associated with the
    /// modules and removes them from the Registry without executing
    /// any additional module code.
    ///
    /// This function should only be used as a recovery mechanism
    /// when normal shutdown has failed.
    pub fn kill_children(entity_id: &EntityID) -> Result<(), FrameworkError> {

        MMF::kill_module_list(&MMF::get_children_ids(entity_id)?)
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
        let normal_sender_channel = mmf_data.registry.get_module_normal_channel(receiver_id)?;

        // Generate the unique id and build the message
        let message_instance_id = MMF::generate_message_instance_id()?;
        let framework_message = FrameworkMessage::new(sender_id.clone(), receiver_id.clone(), message_id.clone(), message_instance_id, message_data);

        // Release MMF lock before awaiting.
        drop(mmf_data_guard);

        // Send the message to the target module.
        normal_sender_channel.send(ModuleCommand::Message(framework_message)).await.map_err(|_| FrameworkError::Module_ChannelClosed)?;

        Ok(())

    }


    /// Send a message and wait until the receiver has finished
    /// processing it.
    ///
    /// timeout_ms:
    /// - None => wait indefinitely.
    /// - Some(value) => return Operation_Timeout if the delay expires.
    pub async fn send_message_and_wait_reply(sender_id: &EntityID, receiver_id: &ModuleID, message_id: &MessageID, message_data: Bytes, timeout_ms: Option<u64>) -> Result<(), FrameworkError> {

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
        let normal_sender_channel = mmf_data.registry.get_module_normal_channel(receiver_id)?;
        drop(mmf_data_guard);

        // Generate the unique id and build the message
        let message_instance_id = MMF::generate_message_instance_id()?;
        let framework_message = FrameworkMessage::new(sender_id.clone(), receiver_id.clone(), message_id.clone(), message_instance_id, message_data);

        // Create a temporary reply channel used to signal
        // completion of on_message().
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel();

        // Send the message to the receiver module.
        normal_sender_channel.send(ModuleCommand::MessageAndWaitReply(reply_sender, framework_message)).await.map_err(|_| FrameworkError::Module_ChannelClosed)?;

        // Wait for completion or timeout.
        match timeout_ms {
            Some(timeout_ms) => {
                tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), reply_receiver).await.map_err(|_| FrameworkError::Operation_Timeout)?.map_err(|_| FrameworkError::Module_ChannelClosed)?;
            }
            None => {
                reply_receiver.await.map_err(|_| FrameworkError::Module_ChannelClosed)?;
            }
        }

        Ok(())
    }


    /// Send a priority message to a module.
    pub async fn send_priority_message(sender_id: &EntityID, receiver_id: &ModuleID, message_id: &MessageID, message_data: Bytes) -> Result<(), FrameworkError> {
        
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
        let priority_sender_channel = mmf_data.registry.get_module_priority_channel(receiver_id)?;

        // Generate the unique id and build the message
        let message_instance_id = MMF::generate_message_instance_id()?;
        let framework_message = FrameworkMessage::new(sender_id.clone(), receiver_id.clone(), message_id.clone(), message_instance_id, message_data);

        // Release MMF lock before awaiting.
        drop(mmf_data_guard);

        // Send the priority message to the target module.
        priority_sender_channel.send(ModuleCommand::Message(framework_message)).await.map_err(|_| FrameworkError::Module_ChannelClosed)?;

        Ok(())

    }

}

