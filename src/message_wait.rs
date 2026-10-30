//! Internal message wait and flow management.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

use crate::channel::{WorkerCommand, WorkerSenderChannel};
use crate::framework_error::{MWFError, MWFPanic, WorkerError};
use crate::message::{MessageFlowID, MessageID, MessageInheritanceContext, MessageInstanceID, MessageWaitMode, WorkerMessage};
use crate::mwf::MWF_MESSAGE_WAIT_DATA;
use crate::worker::{EntityID, WorkerID};

/// Runtime state associated with an active message flow.
#[derive(Debug)]
pub(crate) struct MessageFlowState {
    /// Worker that owns and executes this message flow.
    pub(crate) worker_id: WorkerID,

    /// Channel used to return the flow result to its waiter.
    pub(crate) reply_sender: oneshot::Sender<Result<(), MWFError>>,
}

/// Information about a worker currently waiting for message completion.
#[derive(Debug)]
pub(crate) struct WorkerWaitState {
    /// Worker receiving the awaited message.
    pub(crate) receiver_id: WorkerID,

    /// Functional identifier of the awaited message.
    pub(crate) message_id: MessageID,

    /// Unique identifier of the awaited message instance.
    pub(crate) message_instance_id: MessageInstanceID,

    /// Type of completion being awaited.
    pub(crate) _wait_mode: MessageWaitMode,

    /// Time when the wait started.
    pub(crate) _started_at: Instant,
}

/// Internal data used to manage message waits and flows.
#[derive(Debug)]
pub(crate) struct MessageWaitData {
    /// Active message flows and their reply channels.
    pub(crate) message_flow_registry: HashMap<MessageFlowID, MessageFlowState>,

    /// Workers currently waiting for message completion.
    pub(crate) waiting_worker_registry: HashMap<WorkerID, WorkerWaitState>,
}

impl MessageWaitData {
    /// Create an empty message wait registry.
    pub(crate) fn new() -> Self {
        Self { message_flow_registry: HashMap::new(), waiting_worker_registry: HashMap::new() }
    }
}

/// Returns whether a message flow is still active for the specified worker.
///
/// A missing workflow is treated as inactive so stale message contexts
/// can be safely ignored.
pub(crate) fn is_message_flow_active_for_worker(message_flow_id: MessageFlowID, worker_id: &WorkerID) -> bool {
    let message_wait_data_guard = MWF_MESSAGE_WAIT_DATA.read().expect(MWFPanic::MWFMessageWaitData_ReadLockPoisoned.message());

    message_wait_data_guard.as_ref().and_then(|message_wait_data| message_wait_data.message_flow_registry.get(&message_flow_id)).is_some_and(|message_flow_state| &message_flow_state.worker_id == worker_id)
}

/// Register a worker wait after checking that it cannot create a deadlock cycle.
///
/// The cycle check and registration are performed under the same write lock
/// so concurrent wait requests cannot create a cycle between workers.
pub(crate) fn register_worker_wait(sender_id: &WorkerID, wait_state: WorkerWaitState) -> Result<WorkerWaitGuard, MWFError> {
    // A worker cannot wait for a message sent to itself.
    if sender_id == &wait_state.receiver_id {
        return Err(MWFError::Worker_CannotWaitForSelf);
    }

    let message_instance_id = wait_state.message_instance_id;

    // Lock the registry to check and register the wait atomically.
    let mut message_wait_data_guard = MWF_MESSAGE_WAIT_DATA.write().expect(MWFPanic::MWFMessageWaitData_WriteLockPoisoned.message());

    let message_wait_data = message_wait_data_guard.as_mut().ok_or(MWFError::Framework_NotInitialized)?;

    // Reject the request if the sender already has an active wait.
    if let Some(active_wait) = message_wait_data.waiting_worker_registry.get(sender_id) {
        return Err(MWFError::Worker_AlreadyWaiting { sender_id: sender_id.clone(), waiting_receiver_id: active_wait.receiver_id.clone(), waiting_message_id: active_wait.message_id.clone() });
    }

    // Reject the request if the receiver's existing wait chain leads back to the sender.
    if let Some(wait_chain) = find_wait_cycle(sender_id, &wait_state.receiver_id, &message_wait_data.waiting_worker_registry) {
        // The receiver already has an active wait that would form a cycle.
        let waiting_worker_id = wait_state.receiver_id.clone();

        let active_wait = match message_wait_data.waiting_worker_registry.get(&waiting_worker_id) {
            Some(active_wait) => active_wait,
            None => {
                return Err(MWFError::MessageWait_RegistryInconsistent);
            }
        };

        return Err(MWFError::Worker_WaitWouldDeadlock { sender_id: sender_id.clone(), waiting_receiver_id: active_wait.receiver_id.clone(), waiting_message_id: active_wait.message_id.clone(), wait_chain });
    }

    // Register the wait only after all checks have succeeded.
    message_wait_data.waiting_worker_registry.insert(sender_id.clone(), wait_state);

    // Release the lock before the caller performs any asynchronous operation.
    drop(message_wait_data_guard);

    Ok(WorkerWaitGuard { worker_id: sender_id.clone(), message_instance_id })
}

/// Find a cycle that would be created by adding `sender_id -> receiver_id`.
///
/// Each registered worker has at most one outgoing wait, so the search follows
/// a single chain instead of traversing a general dependency graph.
fn find_wait_cycle(sender_id: &WorkerID, receiver_id: &WorkerID, waiting_worker_registry: &HashMap<WorkerID, WorkerWaitState>) -> Option<Vec<WorkerID>> {
    let mut wait_chain = vec![sender_id.clone()];
    let mut visited_worker_ids = HashSet::new();
    let mut current_worker_id = receiver_id.clone();

    loop {
        // The chain returns to the sender: adding the wait would create a cycle.
        if &current_worker_id == sender_id {
            wait_chain.push(current_worker_id);
            return Some(wait_chain);
        }

        // Stop safely if an existing inconsistent cycle is encountered.
        if !visited_worker_ids.insert(current_worker_id.clone()) {
            wait_chain.push(current_worker_id);
            return Some(wait_chain);
        }

        wait_chain.push(current_worker_id.clone());

        // Follow the current worker's existing wait, if any.
        let wait_state = waiting_worker_registry.get(&current_worker_id)?;
        current_worker_id = wait_state.receiver_id.clone();
    }
}

/// Register a new message flow before submitting its initial message.
pub(crate) fn register_message_flow(message_flow_id: MessageFlowID, worker_id: WorkerID, reply_sender: oneshot::Sender<Result<(), MWFError>>) -> Result<MessageFlowRegistrationGuard, MWFError> {
    let mut message_wait_data_guard = MWF_MESSAGE_WAIT_DATA.write().expect(MWFPanic::MWFMessageWaitData_WriteLockPoisoned.message());

    let message_wait_data = message_wait_data_guard.as_mut().ok_or(MWFError::Framework_NotInitialized)?;

    // A generated flow ID must never replace an existing flow.
    if message_wait_data.message_flow_registry.contains_key(&message_flow_id) {
        return Err(MWFError::MessageFlow_RegistryInconsistent);
    }

    message_wait_data.message_flow_registry.insert(message_flow_id, MessageFlowState { worker_id: worker_id.clone(), reply_sender });

    drop(message_wait_data_guard);

    Ok(MessageFlowRegistrationGuard { message_flow_id, worker_id, armed: true })
}

/// Complete a message flow and notify its caller.
pub(crate) fn complete_message_flow(message_flow_id: MessageFlowID, worker_id: &WorkerID, result: Result<(), WorkerError>) -> Result<(), MWFError> {
    let flow_state = take_message_flow(message_flow_id, worker_id)?;

    // The caller may already have stopped waiting, for example because of a timeout.
    let _ = flow_state.reply_sender.send(result.map_err(MWFError::Worker_MessageError));

    Ok(())
}

/// Remove a message flow and return its state.
///
/// Only the worker that owns the flow may remove it.
fn take_message_flow(message_flow_id: MessageFlowID, worker_id: &WorkerID) -> Result<MessageFlowState, MWFError> {
    let mut message_wait_data_guard = MWF_MESSAGE_WAIT_DATA.write().expect(MWFPanic::MWFMessageWaitData_WriteLockPoisoned.message());

    let message_wait_data = message_wait_data_guard.as_mut().ok_or(MWFError::Framework_NotInitialized)?;

    // Validate that the flow exists and belongs to the current worker.
    let flow_state = message_wait_data.message_flow_registry.get(&message_flow_id).ok_or(MWFError::MessageFlow_NotActive)?;

    if &flow_state.worker_id != worker_id {
        return Err(MWFError::MessageFlow_NotActive);
    }

    // Remove the entry while still holding the write lock.
    message_wait_data.message_flow_registry.remove(&message_flow_id).ok_or(MWFError::MessageFlow_NotActive)
}

/// Removes all active message flows owned by a worker task and notifies their waiters.
///
/// The registry lock is released before notifying waiters.
pub(crate) fn cleanup_worker_task_message_flows(worker_id: &WorkerID) -> Result<(), MWFError> {
    // Remove all flows owned by this worker while holding the registry lock.
    let flow_states = {
        let mut message_wait_data_guard = MWF_MESSAGE_WAIT_DATA.write().map_err(|_| MWFError::MessageWait_RegistryInconsistent)?;

        // Nothing to clean up if the framework data has already been destroyed.
        let Some(message_wait_data) = message_wait_data_guard.as_mut() else {
            return Ok(());
        };

        let message_flow_ids = message_wait_data.message_flow_registry.iter().filter_map(|(message_flow_id, flow_state)| (&flow_state.worker_id == worker_id).then_some(*message_flow_id)).collect::<Vec<_>>();

        message_flow_ids.into_iter().filter_map(|message_flow_id| message_wait_data.message_flow_registry.remove(&message_flow_id)).collect::<Vec<_>>()
    };

    // Notify waiters only after releasing the registry lock.
    for flow_state in flow_states {
        let _ = flow_state.reply_sender.send(Err(MWFError::MessageFlow_WorkerTaskTerminated { worker_id: worker_id.clone() }));
    }

    Ok(())
}

/// Send a message and wait until its callback has completed.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn wait_this_message<TMessageData>(sender_channel: WorkerSenderChannel, sender_id: &EntityID, receiver_id: &WorkerID, message_id: &MessageID, message_instance_id: MessageInstanceID, message_data: &TMessageData, timeout_ms: Option<u64>, priority: bool) -> Result<(), MWFError>
where
    TMessageData: Clone + Send + Sync + 'static,
{
    // Messages waiting for a callback do not inherit a workflow.
    let framework_message = WorkerMessage::new(sender_id, receiver_id, message_id, message_instance_id, message_data, MessageInheritanceContext::default());

    let (reply_sender, reply_receiver) = oneshot::channel::<Result<(), WorkerError>>();

    // Submit a message whose completion is reported by the worker task.
    sender_channel.send(WorkerCommand::MessageAndWaitCompletion { reply_sender, message: framework_message, priority }).await.map_err(|_| MWFError::Worker_ChannelClosed)?;

    // Wait for the callback result or the timeout.
    let worker_result = match timeout_ms {
        Some(timeout_ms) => match tokio::time::timeout(Duration::from_millis(timeout_ms), reply_receiver).await {
            Ok(Ok(worker_result)) => worker_result,
            Ok(Err(_)) => return Err(MWFError::Worker_ChannelClosed),
            Err(_) => return Err(MWFError::Operation_Timeout),
        },
        None => match reply_receiver.await {
            Ok(worker_result) => worker_result,
            Err(_) => return Err(MWFError::Worker_ChannelClosed),
        },
    };

    worker_result.map_err(MWFError::Worker_MessageError)
}

/// Send a message and wait until its entire message flow has completed.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn wait_message_flow<TMessageData>(
    sender_channel: WorkerSenderChannel,
    sender_id: &EntityID,
    receiver_id: &WorkerID,
    message_id: &MessageID,
    message_instance_id: MessageInstanceID,
    message_flow_id: MessageFlowID,
    message_data: &TMessageData,
    timeout_ms: Option<u64>,
    priority: bool,
) -> Result<(), MWFError>
where
    TMessageData: Clone + Send + Sync + 'static,
{
    // Register the new flow before submitting its initial message.
    let (flow_reply_sender, flow_reply_receiver) = oneshot::channel::<Result<(), MWFError>>();

    let mut flow_registration_guard = register_message_flow(message_flow_id, receiver_id.clone(), flow_reply_sender)?;

    // Associate the new flow with its initial message.
    let inheritance_context = MessageInheritanceContext { message_flow_id: Some(message_flow_id) };

    let framework_message = WorkerMessage::new(sender_id, receiver_id, message_id, message_instance_id, message_data, inheritance_context);

    // Submit a regular message: workflow_end() reports completion.
    sender_channel.send(WorkerCommand::Message { message: framework_message, priority, unique: false }).await.map_err(|_| MWFError::Worker_ChannelClosed)?;

    // Preserve the registration after successful submission.
    flow_registration_guard.disarm();

    // Wait for the result explicitly provided by workflow_end().
    match timeout_ms {
        Some(timeout_ms) => match tokio::time::timeout(Duration::from_millis(timeout_ms), flow_reply_receiver).await {
            Ok(Ok(worker_result)) => worker_result,
            Ok(Err(_)) => Err(MWFError::Worker_ChannelClosed),
            Err(_) => Err(MWFError::Operation_Timeout),
        },
        None => match flow_reply_receiver.await {
            Ok(worker_result) => worker_result,
            Err(_) => Err(MWFError::Worker_ChannelClosed),
        },
    }
}

/// Automatically removes a registered wait when the guard is dropped.
#[derive(Debug)]
pub(crate) struct WorkerWaitGuard {
    /// Worker whose wait is registered.
    worker_id: WorkerID,

    /// Message instance associated with this specific wait.
    message_instance_id: MessageInstanceID,
}

impl Drop for WorkerWaitGuard {
    fn drop(&mut self) {
        // Do not panic during cleanup if the registry lock is poisoned.
        let Ok(mut message_wait_data_guard) = MWF_MESSAGE_WAIT_DATA.write() else {
            return;
        };

        let Some(message_wait_data) = message_wait_data_guard.as_mut() else {
            return;
        };

        // Remove only the wait associated with this guard.
        // This prevents an old guard from removing a newer wait.
        let should_remove = message_wait_data.waiting_worker_registry.get(&self.worker_id).is_some_and(|wait_state| wait_state.message_instance_id == self.message_instance_id);

        if should_remove {
            message_wait_data.waiting_worker_registry.remove(&self.worker_id);
        }
    }
}

/// Automatically removes a newly registered flow if message submission fails.
#[derive(Debug)]
pub(crate) struct MessageFlowRegistrationGuard {
    /// Identifier of the registered message flow.
    message_flow_id: MessageFlowID,

    /// Worker that owns this message flow.
    worker_id: WorkerID,

    /// Indicates whether the registration still needs cleanup.
    armed: bool,
}

impl MessageFlowRegistrationGuard {
    /// Preserve the registration after the initial message has been submitted.
    pub(crate) fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for MessageFlowRegistrationGuard {
    fn drop(&mut self) {
        // Cleanup is needed only while submission has not been confirmed.
        if !self.armed {
            return;
        }

        // Do not panic during cleanup if the registry lock is poisoned.
        let Ok(mut message_wait_data_guard) = MWF_MESSAGE_WAIT_DATA.write() else {
            return;
        };

        let Some(message_wait_data) = message_wait_data_guard.as_mut() else {
            return;
        };

        // Remove only the flow registered by this guard and owned by this worker.
        let should_remove = message_wait_data.message_flow_registry.get(&self.message_flow_id).is_some_and(|flow_state| flow_state.worker_id == self.worker_id);

        if should_remove {
            message_wait_data.message_flow_registry.remove(&self.message_flow_id);
        }
    }
}

/// Unit tests for message wait-chain validation.
///
/// These tests exercise `find_wait_cycle()` independently from the global MWF runtime.
/// They verify that:
/// - A wait is allowed when the receiver has no active wait.
/// - A chain of waits is allowed when it ends without a cycle.
/// - A direct or indirect cycle is detected before a new wait is registered.
/// - The detected cycle identifies the sequence of workers involved.
///
/// These tests focus on cycle detection. Registration, cleanup and concurrent access
/// to the global wait registry are tested separately.
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    /// Create a worker identifier for testing.
    fn test_worker_id(worker_id: &str) -> WorkerID {
        WorkerID::new(worker_id)
    }

    /// Create a registered wait targeting the specified worker.
    fn test_wait_state(receiver_id: &str) -> WorkerWaitState {
        WorkerWaitState { receiver_id: test_worker_id(receiver_id), message_id: MessageID::new("TestMessage"), message_instance_id: 1, _wait_mode: MessageWaitMode::WaitThisMessage, _started_at: Instant::now() }
    }

    #[test]
    fn no_cycle_when_receiver_is_not_waiting() {
        let waiting_worker_registry = HashMap::new();

        let cycle = find_wait_cycle(&test_worker_id("A"), &test_worker_id("B"), &waiting_worker_registry);

        assert_eq!(cycle, None);
    }

    #[test]
    fn no_cycle_when_wait_chain_ends() {
        let mut waiting_worker_registry = HashMap::new();

        // Existing wait: B -> C.
        waiting_worker_registry.insert(test_worker_id("B"), test_wait_state("C"));

        // Proposed wait: A -> B. The chain ends at C.
        let cycle = find_wait_cycle(&test_worker_id("A"), &test_worker_id("B"), &waiting_worker_registry);

        assert_eq!(cycle, None);
    }

    #[test]
    fn detects_two_worker_cycle() {
        let mut waiting_worker_registry = HashMap::new();

        // Existing wait: A -> B.
        waiting_worker_registry.insert(test_worker_id("A"), test_wait_state("B"));

        // Proposed wait: B -> A.
        let cycle = find_wait_cycle(&test_worker_id("B"), &test_worker_id("A"), &waiting_worker_registry);

        assert_eq!(cycle, Some(vec![test_worker_id("B"), test_worker_id("A"), test_worker_id("B"),]));
    }

    #[test]
    fn detects_multi_worker_cycle() {
        let mut waiting_worker_registry = HashMap::new();

        // Existing waits: A -> B -> C.
        waiting_worker_registry.insert(test_worker_id("A"), test_wait_state("B"));
        waiting_worker_registry.insert(test_worker_id("B"), test_wait_state("C"));

        // Proposed wait: C -> A.
        let cycle = find_wait_cycle(&test_worker_id("C"), &test_worker_id("A"), &waiting_worker_registry);

        assert_eq!(cycle, Some(vec![test_worker_id("C"), test_worker_id("A"), test_worker_id("B"), test_worker_id("C"),]));
    }
}
