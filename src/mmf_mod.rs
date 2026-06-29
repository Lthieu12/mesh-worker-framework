//! Public MMFMod (module and messaging) API.
//!
//! This API exposes the framework entry points used to
//! manage modules and send messages.

use bytes::Bytes;
use std::sync::atomic::Ordering;
use std::time::Instant;

use crate::MMFLog;
use crate::mmf::{MMFConfiguration, MMF_DATA, MMF_MOD_DATA, MMF_MOD_MESSAGE_INSTANCE_COUNTER, MMF_MOD_INIT_NOTIFY, MMF_MOD_CLOSE_NOTIFY};
use crate::mmf_log::{LogAction, LogStatus};
use crate::framework_error::{FrameworkError, MMFPanic};
use crate::module::{Module, ModuleID, EntityID, ModuleState, ModuleStatus, ModuleError, ModuleList};
use crate::module_registry::Registry;
use crate::module_task::spawn_module_task;
use crate::channel::{ModuleCommand, ModuleNormalSenderChannel, ModulePrioritySenderChannel};
use crate::message::{FrameworkMessage, MessageID, MessageInstanceID};

/// Internal MMFMod runtime data.
#[derive(Debug)]
pub(crate) struct ModuleData {

    /// Registered modules and hierarchy information.
    pub(crate) module_registry: Registry,

    /// Capacity of normal module event queues.
    pub(crate) module_normal_channel_capacity: usize,

    /// Capacity of priority module event queues.
    pub(crate) module_priority_channel_capacity: usize,
}

impl ModuleData {

    /// Create a new MMF runtime instance.
    pub(crate) fn new(mmf_configuration: &MMFConfiguration) -> Self {
        
        Self {
            module_registry: Registry::new(),
            module_normal_channel_capacity: mmf_configuration.module_normal_channel_capacity,
            module_priority_channel_capacity: mmf_configuration.module_priority_channel_capacity,
        }
    }

}


pub struct MMFMod;

/// Public MMFMod entry point.
impl MMFMod {

    /// Generate a unique MessageInstanceID.
    fn generate_message_instance_id() -> Result<MessageInstanceID, FrameworkError> {
        
        let current_id = MMF_MOD_MESSAGE_INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed);
        if current_id == u64::MAX {
            return Err(FrameworkError::Message_InstanceIDExhausted);
        }

        Ok(current_id)
    }


    /// Create and register a new module instance.
    ///
    /// The module type must have been previously
    /// registered using MMF::register_module_type().
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

        // Ensure the framework has been initialized.
        let mmf_data_guard = MMF_DATA.read().expect(MMFPanic::MMFData_ReadLockPoisoned.message());
        let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;
        
        // Ensure the module type has been registered.
        if !mmf_data.module_type_registry.contains_module_type(T::module_type_id()) {
            
            return Err(FrameworkError::ModuleType_NotRegistered);
        }
        drop(mmf_data_guard);

        let mut mmf_data_guard = MMF_MOD_DATA.write().expect(MMFPanic::MMFModData_WriteLockPoisoned.message());
        let mmf_data = mmf_data_guard.as_mut().ok_or(FrameworkError::Framework_NotInitialized)?;
        
        // Child modules can only be attached to an
        // initialized parent module.
        //
        // This prevents creating new hierarchy branches
        // while the parent is starting up or shutting down.
        match &father_id {

            EntityID::Module(father_id_ref) => {

                let module_state = mmf_data.module_registry.get_module_state(father_id_ref)?;

                if module_state != ModuleState::Initialized {

                    return Err(FrameworkError::Module_NotInitialized);
                }
            }

            EntityID::External(_) => {}
        }
        
        let module_id = mmf_data.module_registry.generate_module_id(T::module_type_id());

        let action_start_timer = Instant::now();
        
        // Channel used to send Module commands (like Init and Close) and messages to the module.
        let (normal_sender_channel, normal_receiver_channel) = tokio::sync::mpsc::channel(mmf_data.module_normal_channel_capacity);
        // Channel used to send priority commands and messages to the module.
        let (priority_sender_channel, priority_receiver_channel) = tokio::sync::mpsc::channel(mmf_data.module_priority_channel_capacity);

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
        match mmf_data.module_registry.add_module(&module_id, T::module_type_id(), &father_id, normal_sender_channel, priority_sender_channel, join_handle) {

            Ok(()) => {}

            Err((error, join_handle)) => {

                // Abort the spawned Tokio task to avoid leaving
                // an orphan module running.
                join_handle.abort();

                MMFLog::add_log(action_start_timer, &module_id, LogAction::ModuleSpawn, LogStatus::Error)?;

                return Err(error);
            }
        }

        MMFLog::add_log(action_start_timer, &module_id, LogAction::ModuleSpawn, LogStatus::Ok)?;

        drop(mmf_data_guard);

        Ok(module_id)
    }


    /// Get the current status of a module.
    pub fn get_module_status(module_id: &ModuleID) -> Result<ModuleStatus, FrameworkError> {

        let mmf_data_guard = MMF_MOD_DATA.read().expect(MMFPanic::MMFModData_ReadLockPoisoned.message());
        let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;
        let registry_entry = mmf_data.module_registry.get_registry_entry(module_id)?;
        let normal_channel_capacity = mmf_data.module_normal_channel_capacity;
        let priority_channel_capacity = mmf_data.module_priority_channel_capacity;
        let normal_channel_size = normal_channel_capacity - registry_entry.normal_sender_channel.capacity();
        let priority_channel_size = priority_channel_capacity - registry_entry.priority_sender_channel.capacity();
        
        let module_status = ModuleStatus {
            module_state: registry_entry.module_state,
            normal_channel_size,
            normal_channel_capacity,
            priority_channel_size,
            priority_channel_capacity,
        };

        Ok(module_status)
    }


    /// Retrieve information about all modules currently
    /// registered in MMF.
    ///
    /// The returned list contains a snapshot of the Registry
    /// state at the time of the call.
    ///
    /// This function is primarily intended for:
    /// - Debug tools.
    /// - Monitoring interfaces.
    /// - Runtime inspection.
    ///
    /// Returns Framework_NotInitialized if MMF has not
    /// been initialized through MMF::init().
    pub fn get_module_info_list() -> Result<ModuleList, FrameworkError> {

        let mmf_data_guard = MMF_MOD_DATA.read().expect(MMFPanic::MMFModData_ReadLockPoisoned.message());
        let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;

        Ok(ModuleList {module_info_list: mmf_data.module_registry.get_module_info_list()})
    } 


    /// Get the Rust type name associated with a module.
    ///
    /// Returns:
    /// - Framework_NotInitialized if MMF has not been initialized.
    /// - Module_NotFound if the module does not exist.
    pub fn get_module_rust_type_name(module_id: &ModuleID) -> Result<&'static str, FrameworkError> {

        // Get the ModuleTypeID associated with the module.
        let mmf_mod_data_guard = MMF_MOD_DATA.read().expect(MMFPanic::MMFModData_ReadLockPoisoned.message());
        let mmf_mod_data = mmf_mod_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;
        let module_type_id = mmf_mod_data.module_registry.get_module_type_id(module_id)?;
        drop(mmf_mod_data_guard);

        // Get the Rust type name associated with the ModuleTypeID.
        let mmf_data_guard = MMF_DATA.read().expect(MMFPanic::MMFData_ReadLockPoisoned.message());
        let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;
        let module_rust_type_name = mmf_data.module_type_registry.get_module_rust_type_name(module_type_id)?;

        Ok(module_rust_type_name)
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
        let mut mmf_data_guard = MMF_MOD_DATA.write().expect(MMFPanic::MMFModData_WriteLockPoisoned.message());
        let mmf_data = mmf_data_guard.as_mut().ok_or(FrameworkError::Framework_NotInitialized)?;
        let module_state = mmf_data.module_registry.get_module_state(module_id)?;
        if module_state != ModuleState::Created {
            return Err(FrameworkError::Module_AlreadyInitialized);
        }
        mmf_data.module_registry.set_module_state(module_id, ModuleState::Initializing)?;
        let normal_sender_channel = mmf_data.module_registry.get_module_normal_channel(module_id)?;
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
        let mut mmf_data_guard = MMF_MOD_DATA.write().expect(MMFPanic::MMFModData_WriteLockPoisoned.message());
        let mmf_data = mmf_data_guard.as_mut().ok_or(FrameworkError::Framework_NotInitialized)?;
        let module_state = mmf_data.module_registry.get_module_state(module_id)?;
        if module_state != ModuleState::Initialized {
            return Err(FrameworkError::Module_NotInitialized);
        }
        mmf_data.module_registry.set_module_state(module_id, ModuleState::Closing)?;
        let normal_sender_channel = mmf_data.module_registry.get_module_normal_channel(module_id)?;
        drop(mmf_data_guard);

        // Send the Close command to ModuleTask.
        normal_sender_channel.send(ModuleCommand::Close).await.map_err(|_| FrameworkError::Module_ChannelClosed)?;

        Ok(())
    }


    /// Get the FatherID of a module.
    pub fn get_father_id(module_id: &ModuleID) -> Result<EntityID, FrameworkError>{
        
        let mmf_data_guard = MMF_MOD_DATA.read().expect(MMFPanic::MMFModData_ReadLockPoisoned.message());
        let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;
        let father_id = mmf_data.module_registry.get_father_id(module_id)?;
        drop(mmf_data_guard);

        Ok(father_id)
    }


    /// Get the child ModuleIDs of an entity.
    pub fn get_children_ids(entity_id: &EntityID) -> Result<Vec<ModuleID>, FrameworkError> {
        
        let mmf_data_guard = MMF_MOD_DATA.read().expect(MMFPanic::MMFModData_ReadLockPoisoned.message());
        let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;

        mmf_data.module_registry.get_children_ids(entity_id)
    }


    /// Get the number of child modules of an entity.
    pub fn get_children_count(entity_id: &EntityID) -> Result<usize, FrameworkError> {

        let mmf_data_guard = MMF_MOD_DATA.read().expect(MMFPanic::MMFModData_ReadLockPoisoned.message());
        let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;
        let children_count = mmf_data.module_registry.get_children_count(entity_id)?;
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
                let init_notify = MMF_MOD_INIT_NOTIFY.notified();

                // Read Registry state under a short-lived lock.
                //
                // The lock must be released before awaiting the
                // next notification.
                let (all_init_finished, one_init_failed) = {

                    let mmf_data_guard = MMF_MOD_DATA.read().expect(MMFPanic::MMFModData_ReadLockPoisoned.message());
                    let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;

                    let mut all_init_finished = true;
                    let mut one_init_failed = false;

                    // Initialization is complete only when all modules
                    // have reached the Initialized or InitFailed state.
                    for module_id in module_ids {

                        match mmf_data.module_registry.get_module_state(module_id)? {

                            ModuleState::Initialized => {}

                            ModuleState::InitFailed => {
                                one_init_failed = true;
                            }

                            _ => {
                                all_init_finished = false;
                            }
                        }
                    }

                    (all_init_finished, one_init_failed)
                };

                if all_init_finished {

                    if one_init_failed {
                        return Err(FrameworkError::Module_InitializationFailed);
                    }

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

        MMFMod::wait_module_list_init(&vec![module_id.clone()], timeout_ms).await
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
        MMFMod::wait_module_list_init(&MMFMod::get_children_ids(entity_id)?, timeout_ms).await
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

                // Register interest in the next closing event
                // before checking module states to avoid lost notifications.
                let close_notify = MMF_MOD_CLOSE_NOTIFY.notified();

                // Read Registry state under a short-lived lock.
                //
                // The lock must be released before awaiting the
                // next notification.
                let (all_close_finished, one_close_failed) = {

                    let mmf_data_guard = MMF_MOD_DATA.read().expect(MMFPanic::MMFModData_ReadLockPoisoned.message());
                    let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;

                    let mut all_close_finished = true;
                    let mut one_close_failed = false;

                    // Closing is complete only when all modules
                    // have either been removed from the Registry
                    // or reached the CloseFailed or the Panicked state.
                    for module_id in module_ids {

                        match mmf_data.module_registry.get_module_state(module_id) {

                            // A module not found has been properly closed.
                            Err(FrameworkError::Module_NotFound) => {}

                            Ok(ModuleState::CloseFailed) => {
                                one_close_failed = true;
                            }

                            _ => {
                                all_close_finished = false;
                            }
                        }
                    }

                    (all_close_finished, one_close_failed)
                };

                if all_close_finished {

                    if one_close_failed {
                        return Err(FrameworkError::Module_CloseFailed);
                    }

                    return Ok(());
                }

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

        MMFMod::wait_module_list_close(&vec![module_id.clone()], timeout_ms).await
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
        MMFMod::wait_module_list_close(&MMFMod::get_children_ids(entity_id)?, timeout_ms).await
    }


    /// Forcefully terminate a list of modules.
    ///
    /// This operation aborts the Tokio tasks associated with the
    /// modules and removes them from the Registry without executing
    /// any additional module code.
    ///
    /// This function should only be used as a recovery mechanism
    /// when normal shutdown has failed.
    pub fn abort_module_list(module_ids: &Vec<ModuleID>) -> Result<(), FrameworkError> {

        let mut mmf_data_guard = MMF_MOD_DATA.write().expect(MMFPanic::MMFModData_WriteLockPoisoned.message());
        let mmf_data = mmf_data_guard.as_mut().ok_or(FrameworkError::Framework_NotInitialized)?;

        for module_id in module_ids {

            let action_start_timer = Instant::now();
            
            let join_handle = match mmf_data.module_registry.take_join_handle(module_id) {

                Ok(join_handle) => join_handle,

                Err(error) => {

                    MMFLog::add_log(action_start_timer, module_id, LogAction::ModuleAbort, LogStatus::Error)?;
                    return Err(error);
                }
            };

            // Abort the Tokio task immediately.
            join_handle.abort();

            // Remove the module from the Registry.
            if let Err(error) = mmf_data.module_registry.remove_module(module_id) {

                MMFLog::add_log(action_start_timer, module_id, LogAction::ModuleAbort, LogStatus::Error)?;
                return Err(error);
            }

            MMFLog::add_log(action_start_timer, module_id, LogAction::ModuleAbort, LogStatus::Ok)?;
            let _ = MMFLog::remove_logged_module(&module_id);
        }

        drop(mmf_data_guard);

        // Wake up all waiters interested in module
        // shutdown events.
        MMF_MOD_CLOSE_NOTIFY.notify_waiters();

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
    pub fn abort_module(module_id: &ModuleID) -> Result<(), FrameworkError> {

        MMFMod::abort_module_list(&vec![module_id.clone()])
    }


    /// Forcefully terminate all direct children modules of an entity.
    ///
    /// This operation aborts the Tokio tasks associated with the
    /// modules and removes them from the Registry without executing
    /// any additional module code.
    ///
    /// This function should only be used as a recovery mechanism
    /// when normal shutdown has failed.
    pub fn abort_children(entity_id: &EntityID) -> Result<(), FrameworkError> {

        MMFMod::abort_module_list(&MMFMod::get_children_ids(entity_id)?)
    }
 

    /// Send a message to a module.
    pub async fn send_message(sender_id: &EntityID, receiver_id: &ModuleID, message_id: &MessageID, message_data: Bytes) -> Result<(), FrameworkError> {
        
        let normal_sender_channel: ModuleNormalSenderChannel = {
            // Read MMF data.
            //
            // send_message() only requires a read lock because it does
            // not modify the Registry or ModuleState.
            let mmf_data_guard = MMF_MOD_DATA.read().expect(MMFPanic::MMFModData_ReadLockPoisoned.message());

            let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;

            // Validate the sender.
            //
            // MMF modules must be initialized before they can
            // send messages through MMF.
            //
            // External senders are not validated.
            match sender_id {
                EntityID::Module(module_id) => {
                    let module_state = mmf_data.module_registry.get_module_state(module_id)?;
                    if module_state != ModuleState::Initialized {
                        return Err(FrameworkError::Module_NotInitialized);
                    }
                }
                EntityID::External(_) => {}
            }

            // Validate the receiver module state.
            //
            // Messages can only be sent to initialized modules.
            let module_state = mmf_data.module_registry.get_module_state(receiver_id)?;
            if module_state != ModuleState::Initialized {
                return Err(FrameworkError::Module_NotInitialized);
            }

            // Retrieve the communication channel associated with
            // the receiver module.
            //
            // State validation and channel retrieval are performed
            // under the same lock to avoid race conditions with
            // close_module().
            mmf_data.module_registry.get_module_normal_channel(receiver_id)?
        };

        // Generate the unique id and build the message
        let message_instance_id = MMFMod::generate_message_instance_id()?;
        let framework_message = FrameworkMessage::new(sender_id.clone(), receiver_id.clone(), message_id.clone(), message_instance_id, message_data);

        // Send the message to the target module.
        normal_sender_channel.send(ModuleCommand::Message {message: framework_message, priority: false}).await.map_err(|_| FrameworkError::Module_ChannelClosed)?;

        Ok(())

    }


    /// Send a message and wait until the receiver has finished
    /// processing it.
    ///
    /// Returns:
    /// - Ok(()) if on_message() completed successfully.
    /// - ModuleError if on_message() returned an error.
    /// - Operation_Timeout if the timeout expires.
    pub async fn send_message_and_wait(sender_id: &EntityID, receiver_id: &ModuleID, message_id: &MessageID, message_data: Bytes, timeout_ms: Option<u64>) -> Result<(), FrameworkError> {

        let normal_sender_channel: ModuleNormalSenderChannel = {
            // Read MMF data.
            //
            // send_message() only requires a read lock because it does
            // not modify the Registry or ModuleState.
            let mmf_data_guard = MMF_MOD_DATA.read().expect(MMFPanic::MMFModData_ReadLockPoisoned.message());

            let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;

            // Validate the sender.
            //
            // MMF modules must be initialized before they can
            // send messages through MMF.
            //
            // External senders are not validated.
            match sender_id {
                EntityID::Module(module_id) => {
                    let module_state = mmf_data.module_registry.get_module_state(module_id)?;
                    if module_state != ModuleState::Initialized {
                        return Err(FrameworkError::Module_NotInitialized);
                    }
                }
                EntityID::External(_) => {}
            }

            // Validate the receiver module state.
            //
            // Messages can only be sent to initialized modules.
            let module_state = mmf_data.module_registry.get_module_state(receiver_id)?;
            if module_state != ModuleState::Initialized {
                return Err(FrameworkError::Module_NotInitialized);
            }

            // Retrieve the communication channel associated with
            // the receiver module.
            //
            // State validation and channel retrieval are performed
            // under the same lock to avoid race conditions with
            // close_module().
            mmf_data.module_registry.get_module_normal_channel(receiver_id)?
        };

        // Generate the unique id and build the message
        let message_instance_id = MMFMod::generate_message_instance_id()?;
        let framework_message = FrameworkMessage::new(sender_id.clone(), receiver_id.clone(), message_id.clone(), message_instance_id, message_data);

        // Create a temporary reply channel used to signal
        // completion of on_message().
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<Result<(), ModuleError>>();

        // Send the message to the receiver module.
        normal_sender_channel.send(ModuleCommand::MessageAndWaitCompletion{reply_sender, message: framework_message, priority: false}).await.map_err(|_| FrameworkError::Module_ChannelClosed)?;

        // Wait for completion or timeout.
        //
        // The reply channel transports the Result returned
        // by the receiver on_message() callback.
        let module_result: Result<(), ModuleError> =
            match timeout_ms {

                Some(timeout_ms) => {

                    // Wait until the receiver module finishes
                    // processing the message.
                    //
                    // timeout() returns:
                    // - Ok(channel_result) if the receiver replied.
                    // - Err(Elapsed) if the timeout expires.
                    let timeout_result = tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), reply_receiver).await;

                    // Convert timeout errors into MMF errors.
                    let channel_result = match timeout_result {

                        Ok(channel_result) => {
                            channel_result
                        }
                        Err(_) => {
                            return Err(FrameworkError::Operation_Timeout);
                        }
                    };

                    // The receiver module task has disappeared
                    // before sending its completion status.
                    //
                    // channel_result contains:
                    // - Ok(module_result)
                    // - Err(RecvError)
                    match channel_result {

                        Ok(module_result) => {
                            module_result
                        }
                        Err(_) => {
                            return Err(FrameworkError::Module_ChannelClosed);
                        }
                    }
                }

                None => {

                    // Wait indefinitely until the receiver module
                    // finishes processing the message.
                    let channel_result = reply_receiver.await;

                    // The receiver module task has disappeared
                    // before sending its completion status.
                    match channel_result {

                        Ok(module_result) => {
                            module_result
                        }
                        Err(_) => {
                            return Err(FrameworkError::Module_ChannelClosed);
                        }
                    }
                }
            };

        // The receiver module returned an error while
        // processing the message.
        match module_result {

            Ok(()) => {}
            Err(module_error) => {
                return Err(FrameworkError::Module_MessageError(module_error));
            }
        }

        Ok(())
    }


    /// Send a priority message to a module.
    pub async fn send_priority_message(sender_id: &EntityID, receiver_id: &ModuleID, message_id: &MessageID, message_data: Bytes) -> Result<(), FrameworkError> {

        let priority_sender_channel: ModulePrioritySenderChannel = {    
            // Read MMF data.
            //
            // send_message() only requires a read lock because it does
            // not modify the Registry or ModuleState.
            let mmf_data_guard = MMF_MOD_DATA.read().expect(MMFPanic::MMFModData_ReadLockPoisoned.message());

            let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;

            // Validate the sender.
            //
            // MMF modules must be initialized before they can
            // send messages through MMF.
            //
            // External senders are not validated.
            match sender_id {
                EntityID::Module(module_id) => {
                    let module_state = mmf_data.module_registry.get_module_state(module_id)?;
                    if module_state != ModuleState::Initialized {
                        return Err(FrameworkError::Module_NotInitialized);
                    }
                }
                EntityID::External(_) => {}
            }

            // Validate the receiver module state.
            //
            // Messages can only be sent to initialized modules.
            let module_state = mmf_data.module_registry.get_module_state(receiver_id)?;
            if module_state != ModuleState::Initialized {
                return Err(FrameworkError::Module_NotInitialized);
            }

            // Retrieve the communication channel associated with
            // the receiver module.
            //
            // State validation and channel retrieval are performed
            // under the same lock to avoid race conditions with
            // close_module().
            mmf_data.module_registry.get_module_priority_channel(receiver_id)?
        };

        // Generate the unique id and build the message
        let message_instance_id = MMFMod::generate_message_instance_id()?;
        let framework_message = FrameworkMessage::new(sender_id.clone(), receiver_id.clone(), message_id.clone(), message_instance_id, message_data);

        // Send the priority message to the target module.
        priority_sender_channel.send(ModuleCommand::Message {message: framework_message, priority: true}).await.map_err(|_| FrameworkError::Module_ChannelClosed)?;

        Ok(())

    }


    /// Send a priority message and wait until the receiver has finished
    /// processing it.
    ///
    /// Returns:
    /// - Ok(()) if on_message() completed successfully.
    /// - ModuleError if on_message() returned an error.
    /// - Operation_Timeout if the timeout expires.
    pub async fn send_priority_message_and_wait(sender_id: &EntityID, receiver_id: &ModuleID, message_id: &MessageID, message_data: Bytes, timeout_ms: Option<u64>) -> Result<(), FrameworkError> {

        let priority_sender_channel: ModulePrioritySenderChannel = {
            // Read MMF data.
            //
            // send_message() only requires a read lock because it does
            // not modify the Registry or ModuleState.
            let mmf_data_guard = MMF_MOD_DATA.read().expect(MMFPanic::MMFModData_ReadLockPoisoned.message());

            let mmf_data = mmf_data_guard.as_ref().ok_or(FrameworkError::Framework_NotInitialized)?;

            // Validate the sender.
            //
            // MMF modules must be initialized before they can
            // send messages through MMF.
            //
            // External senders are not validated.
            match sender_id {
                EntityID::Module(module_id) => {
                    let module_state = mmf_data.module_registry.get_module_state(module_id)?;
                    if module_state != ModuleState::Initialized {
                        return Err(FrameworkError::Module_NotInitialized);
                    }
                }
                EntityID::External(_) => {}
            }

            // Validate the receiver module state.
            //
            // Messages can only be sent to initialized modules.
            let module_state = mmf_data.module_registry.get_module_state(receiver_id)?;
            if module_state != ModuleState::Initialized {
                return Err(FrameworkError::Module_NotInitialized);
            }

            // Retrieve the communication channel associated with
            // the receiver module.
            //
            // State validation and channel retrieval are performed
            // under the same lock to avoid race conditions with
            // close_module().
            mmf_data.module_registry.get_module_priority_channel(receiver_id)?
        };

        // Generate the unique id and build the message
        let message_instance_id = MMFMod::generate_message_instance_id()?;
        let framework_message = FrameworkMessage::new(sender_id.clone(), receiver_id.clone(), message_id.clone(), message_instance_id, message_data);

        // Create a temporary reply channel used to signal
        // completion of on_message().
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel();

        // Send the message to the receiver module.
        priority_sender_channel.send(ModuleCommand::MessageAndWaitCompletion {reply_sender, message: framework_message, priority: true}).await.map_err(|_| FrameworkError::Module_ChannelClosed)?;

        // Wait for completion or timeout.
        //
        // The reply channel transports the Result returned
        // by the receiver on_message() callback.
        let module_result: Result<(), ModuleError> =
            match timeout_ms {

                Some(timeout_ms) => {

                    // Wait until the receiver module finishes
                    // processing the message.
                    //
                    // timeout() returns:
                    // - Ok(channel_result) if the receiver replied.
                    // - Err(Elapsed) if the timeout expires.
                    let timeout_result = tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), reply_receiver).await;

                    // Convert timeout errors into MMF errors.
                    let channel_result = match timeout_result {

                        Ok(channel_result) => {
                            channel_result
                        }
                        Err(_) => {
                            return Err(FrameworkError::Operation_Timeout);
                        }
                    };

                    // The receiver module task has disappeared
                    // before sending its completion status.
                    //
                    // channel_result contains:
                    // - Ok(module_result)
                    // - Err(RecvError)
                    match channel_result {

                        Ok(module_result) => {
                            module_result
                        }
                        Err(_) => {
                            return Err(FrameworkError::Module_ChannelClosed);
                        }
                    }
                }

                None => {

                    // Wait indefinitely until the receiver module
                    // finishes processing the message.
                    let channel_result = reply_receiver.await;

                    // The receiver module task has disappeared
                    // before sending its completion status.
                    match channel_result {

                        Ok(module_result) => {
                            module_result
                        }
                        Err(_) => {
                            return Err(FrameworkError::Module_ChannelClosed);
                        }
                    }
                }
            };

        // The receiver module returned an error while
        // processing the message.
        match module_result {

            Ok(()) => {}
            Err(module_error) => {
                return Err(FrameworkError::Module_MessageError(module_error));
            }
        }

        Ok(())
    }

}