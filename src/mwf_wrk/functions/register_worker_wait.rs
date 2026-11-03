use std::collections::{HashMap, HashSet};

use crate::mwf::MWF_MESSAGE_WAIT_DATA;
use crate::mwf::{MWFError, MWFPanic};
use crate::mwf_wrk::types::{WorkerID, WorkerWaitGuard, WorkerWaitState};

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
    use crate::{MessageID, MessageWaitMode};
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
