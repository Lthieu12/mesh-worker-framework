//! Validate runtime deadlock detection and wait registration cleanup.
//!
//! Scenario:
//! - Create two workers that wait for each other.
//! - Verify that MWF rejects the circular wait.
//! - Repeat the operation to verify that the previous wait was cleaned up.
//! - Shutdown both workers and MWF.

use serial_test::serial;

use dev_support::simulation_worker::{SimulationBehavior, SimulationWorker, SimulationWorkerConfig};
use mwf::*;

/// Check whether an MWF error contains a deadlock detection error.
fn is_wait_would_deadlock(error: &MWFError) -> bool {
    match error {
        MWFError::Worker_WaitWouldDeadlock { .. } => true,

        MWFError::Worker_MessageError(worker_error) => worker_error.downcast_ref::<MWFError>().is_some_and(is_wait_would_deadlock),

        _ => false,
    }
}

// MWF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_worker_wait_cycle() {
    MWF::init().unwrap();
    MWF::register_worker_type::<SimulationWorker>().unwrap();

    let parent_id = EntityID::new_external("MAIN");

    // Spawn both workers.
    let worker_a_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &parent_id).unwrap();
    let worker_b_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &parent_id).unwrap();

    // Worker A waits for B.
    let configuration_a = SimulationWorkerConfig {
        init_delay_ms: 0,
        close_delay_ms: 0,
        simulation_behavior: SimulationBehavior::Message_SendAndWait { target_worker_id: worker_b_id.clone(), message_id: MessageID::new("CYCLE"), timeout_ms: Some(1000), message_delay_ms: 0 },
        wait_children_close: false,
        wait_children_close_timeout_ms: None,
    };

    // Worker B uses the priority send-and-wait path to attempt
    // the reverse wait, which would create the cycle A -> B -> A.
    let configuration_b = SimulationWorkerConfig {
        init_delay_ms: 0,
        close_delay_ms: 0,
        simulation_behavior: SimulationBehavior::Message_SendPriorityAndWait { target_worker_id: worker_a_id.clone(), message_id: MessageID::new("CYCLE"), timeout_ms: Some(1000), message_delay_ms: 0 },
        wait_children_close: false,
        wait_children_close_timeout_ms: None,
    };

    // Initialize both workers.
    MWFWrk::init_worker(&worker_a_id, &configuration_a).await.unwrap();
    MWFWrk::init_worker(&worker_b_id, &configuration_b).await.unwrap();

    let worker_ids = vec![worker_a_id.clone(), worker_b_id.clone()];
    MWFWrk::wait_worker_list_init(&worker_ids, Some(1000)).await.unwrap();

    // Trigger the A -> B -> A wait sequence.
    let test_sender_id = EntityID::new_external("TEST");
    let trigger_message_id = MessageID::new("TRIGGER");

    let first_result = MWFWrk::send_message_and_wait(&test_sender_id, &worker_a_id, &trigger_message_id, &(), Some(3000)).await;

    // Repeat the cycle. A stale registration would reject A's new wait
    // with Worker_AlreadyWaiting instead of detecting the circular wait.
    let second_result = MWFWrk::send_message_and_wait(&test_sender_id, &worker_a_id, &trigger_message_id, &(), Some(3000)).await;

    // Close both workers before asserting, so framework cleanup still runs
    // if an assertion below fails.
    MWFWrk::close_worker(&worker_a_id, &()).await.unwrap();
    MWFWrk::close_worker(&worker_b_id, &()).await.unwrap();
    MWFWrk::wait_worker_list_close(&worker_ids, Some(1000)).await.unwrap();

    MWF::close().unwrap();

    // Both attempts must fail because of the circular wait, not a stale wait entry.
    let first_error = first_result.unwrap_err();
    assert!(is_wait_would_deadlock(&first_error), "Expected Worker_WaitWouldDeadlock on the first attempt, got: {first_error:?}");

    let second_error = second_result.unwrap_err();
    assert!(is_wait_would_deadlock(&second_error), "Expected Worker_WaitWouldDeadlock on the second attempt, got: {second_error:?}");
}
