//! Worker registry for MWF.
//!
//! This file stores all registered workers, their hierarchy,
//! their execution state and generates unique WorkerIDs.

use std::collections::HashMap;
use tokio::task::JoinHandle;

use crate::channel::{WorkerNormalSenderChannel, WorkerPrioritySenderChannel};
use crate::framework_error::{MWFError, MWFPanic};
use crate::worker::{EntityID, WorkerID, WorkerInfo, WorkerState, WorkerTypeID};

/// Information associated with a registered worker.
#[derive(Debug)]
pub(crate) struct RegistryEntry {
    /// Public identifier of the worker type.
    pub(crate) worker_type_id: WorkerTypeID,

    /// Identifier of the entity that spawned this worker.
    parent_id: EntityID,

    /// Current worker state.
    pub(crate) worker_state: WorkerState,

    /// Channel used to send commands to the worker.
    pub(crate) normal_sender_channel: WorkerNormalSenderChannel,

    /// Channel used to send priority commands to the worker.
    pub(crate) priority_sender_channel: WorkerPrioritySenderChannel,

    /// Tokio task handle associated with the worker runtime.
    join_handle: Option<JoinHandle<()>>,
}

impl RegistryEntry {
    /// Create a registry entry for a newly created worker.
    fn new(worker_type_id: WorkerTypeID, parent_id: EntityID, normal_sender_channel: WorkerNormalSenderChannel, priority_sender_channel: WorkerPrioritySenderChannel, join_handle: JoinHandle<()>) -> Self {
        Self { worker_type_id, parent_id, worker_state: WorkerState::Created, normal_sender_channel, priority_sender_channel, join_handle: Some(join_handle) }
    }
}

/// Stores all workers known by the framework.
#[derive(Debug)]
pub(crate) struct Registry {
    /// Maps a WorkerID to its registry entry.
    /// Source of truth for worker existence.
    worker_map: HashMap<WorkerID, RegistryEntry>,

    /// Maps an entity to its child WorkerIDs.
    child_worker_map: HashMap<EntityID, Vec<WorkerID>>,

    /// Stores the instance counter for each worker type.
    worker_counter_map: HashMap<WorkerTypeID, u64>,
}

impl Registry {
    /// Create an empty registry.
    pub(crate) fn new() -> Self {
        Self { worker_map: HashMap::new(), child_worker_map: HashMap::new(), worker_counter_map: HashMap::new() }
    }

    /// Get worker registry entry from WorkerID.
    pub(crate) fn get_registry_entry(&self, worker_id: &WorkerID) -> Result<&RegistryEntry, MWFError> {
        self.worker_map.get(worker_id).ok_or(MWFError::Worker_NotFound)
    }

    /// Get mutable worker registry entry from WorkerID.
    pub(crate) fn get_registry_entry_mut(&mut self, worker_id: &WorkerID) -> Result<&mut RegistryEntry, MWFError> {
        self.worker_map.get_mut(worker_id).ok_or(MWFError::Worker_NotFound)
    }

    /// Retrieve information about all workers currently
    /// present in the Registry.
    ///
    /// The returned list contains one WorkerInfo entry
    /// per registered worker.
    pub(crate) fn get_worker_info_list(&self) -> Vec<WorkerInfo> {
        let mut worker_info_list = Vec::new();

        for (worker_id, registry_entry) in &self.worker_map {
            worker_info_list.push(WorkerInfo { worker_id: worker_id.clone(), parent_id: registry_entry.parent_id.clone(), worker_state: registry_entry.worker_state });
        }

        worker_info_list
    }

    /// Get worker type id from WorkerID.
    pub(crate) fn get_worker_type_id(&self, worker_id: &WorkerID) -> Result<WorkerTypeID, MWFError> {
        Ok(self.get_registry_entry(worker_id)?.worker_type_id)
    }

    /// Get worker channel from WorkerID.
    pub(crate) fn get_worker_normal_channel(&self, worker_id: &WorkerID) -> Result<WorkerNormalSenderChannel, MWFError> {
        Ok(self.get_registry_entry(worker_id)?.normal_sender_channel.clone())
    }

    /// Get worker priority channel from WorkerID.
    pub(crate) fn get_worker_priority_channel(&self, worker_id: &WorkerID) -> Result<WorkerPrioritySenderChannel, MWFError> {
        Ok(self.get_registry_entry(worker_id)?.priority_sender_channel.clone())
    }

    /// Get worker state from WorkerID
    pub(crate) fn get_worker_state(&self, worker_id: &WorkerID) -> Result<WorkerState, MWFError> {
        Ok(self.get_registry_entry(worker_id)?.worker_state)
    }

    /// Generate a unique WorkerID for a worker type.
    pub(crate) fn generate_worker_id(&mut self, worker_type_id: WorkerTypeID) -> WorkerID {
        let counter = self.worker_counter_map.entry(worker_type_id).or_insert(0);
        *counter += 1;

        WorkerID::new(format!("{}:{:05}", worker_type_id, *counter))
    }

    /// Check if the registry is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.worker_map.is_empty()
    }

    /// Register a newly created worker.
    pub(crate) fn add_worker(&mut self, worker_id: &WorkerID, worker_type_id: WorkerTypeID, parent_id: &EntityID, normal_sender_channel: WorkerNormalSenderChannel, priority_sender_channel: WorkerPrioritySenderChannel, join_handle: JoinHandle<()>) -> Result<(), (MWFError, JoinHandle<()>)> {
        // Prevent duplicate WorkerIDs.
        if self.worker_map.contains_key(worker_id) {
            return Err((MWFError::Worker_AlreadyExists, join_handle));
        }

        // Validate the parent entity.
        //
        // MWF workers must already exist before they can be
        // referenced as a parent.
        //
        // External entities are not managed by MWF and therefore
        // cannot be validated.
        match parent_id {
            EntityID::Worker(parent_id_ref) => {
                if !Registry::contains_worker(self, parent_id_ref) {
                    return Err((MWFError::ParentWorker_NotFound, join_handle));
                }
            }
            EntityID::External(_) => {}
        }

        // Create the Registry entry associated with the worker.
        let registry_entry = RegistryEntry::new(worker_type_id, parent_id.clone(), normal_sender_channel, priority_sender_channel, join_handle);
        // Register the worker.
        self.worker_map.insert(worker_id.to_owned(), registry_entry);

        // Register the parent/child relationship.
        //
        // child_worker_map only stores existing relationships.
        //
        // workers without children do not have an entry in this map.
        self.child_worker_map.entry(parent_id.clone()).or_default().push(worker_id.to_owned());

        Ok(())
    }

    /// Remove a worker from the registry.
    pub(crate) fn remove_worker(&mut self, worker_id: &WorkerID) -> Result<(), MWFError> {
        // Retrieve the Registry entry associated with the worker.
        //
        // The parent_id must be retrieved before removing the worker
        // from worker_map.
        let registry_entry = self.get_registry_entry(worker_id)?;
        let parent_id = registry_entry.parent_id.clone();

        // Remove the worker from the Registry.
        self.worker_map.remove(worker_id);

        // Remove the worker from its father's child list.
        //
        // child_worker_map only stores existing parent/child
        // relationships.
        if let Some(child_worker_list) = self.child_worker_map.get_mut(&parent_id) {
            if let Some(index) = child_worker_list.iter().position(|child_id| child_id == worker_id) {
                child_worker_list.swap_remove(index);
            }

            // Remove the father entry if it no longer has children.
            if child_worker_list.is_empty() {
                self.child_worker_map.remove(&parent_id);
            }
        }

        // Remove the child list associated with the closed worker.
        //
        // A closed worker can no longer be the parent of other
        // workers.
        self.child_worker_map.remove(&EntityID::Worker(worker_id.to_owned()));

        Ok(())
    }

    /// Update the worker state of a registered worker.
    pub(crate) fn set_worker_state(&mut self, worker_id: &WorkerID, worker_state: WorkerState) -> Result<(), MWFError> {
        let registry_entry = self.get_registry_entry_mut(worker_id)?;
        registry_entry.worker_state = worker_state;
        Ok(())
    }

    /// Returns true if the worker exists in the Registry.
    pub(crate) fn contains_worker(&self, worker_id: &WorkerID) -> bool {
        self.worker_map.contains_key(worker_id)
    }

    /// Get the worker ParentID from WorkerID.
    pub(crate) fn get_parent_id(&self, worker_id: &WorkerID) -> Result<EntityID, MWFError> {
        let registry_entry = self.get_registry_entry(worker_id)?;
        Ok(registry_entry.parent_id.clone())
    }

    /// Get the child WorkerIDs of an entity.
    pub(crate) fn get_children_ids(&self, entity_id: &EntityID) -> Result<Vec<WorkerID>, MWFError> {
        // Validate the worker if the entity is managed by MWF.
        //
        // External entities are not managed by MWF and therefore
        // cannot be validated.
        if let EntityID::Worker(worker_id) = entity_id {
            if !Registry::contains_worker(self, worker_id) {
                return Err(MWFError::Worker_NotFound);
            }
        }

        // child_worker_map only stores existing parent/child
        // relationships.
        //
        // If the entity has no children, return an empty list.
        match self.child_worker_map.get(entity_id) {
            Some(child_worker_list) => Ok(child_worker_list.clone()),
            None => Ok(Vec::new()),
        }
    }

    /// Get the number of child workers of an entity.
    pub(crate) fn get_children_count(&self, entity_id: &EntityID) -> Result<usize, MWFError> {
        // Validate the worker if the entity is managed by MWF.
        //
        // External entities are not managed by MWF and therefore
        // cannot be validated.
        if let EntityID::Worker(worker_id) = entity_id {
            if !Registry::contains_worker(self, worker_id) {
                return Err(MWFError::Worker_NotFound);
            }
        }

        // child_worker_map only stores existing parent/child
        // relationships.
        //
        // If the entity has no children, return zero.
        match self.child_worker_map.get(entity_id) {
            Some(child_worker_list) => Ok(child_worker_list.len()),
            None => Ok(0),
        }
    }

    /// Take ownership of the JoinHandle associated with a worker.
    pub(crate) fn take_join_handle(&mut self, worker_id: &WorkerID) -> Result<JoinHandle<()>, MWFError> {
        let registry_entry = self.get_registry_entry_mut(worker_id)?;
        let join_handle = registry_entry.join_handle.take().expect(MWFPanic::worker_JoinHandleMissing.message());
        Ok(join_handle)
    }
}
