//! Validate the current worker execution context.
//!
//! Scenario:
//! - Verify that context getters fail outside an MWF worker task.
//! - Create two workers with different external parents.
//! - Verify each worker's current WorkerID and parent ID through a message.
//! - Shutdown both workers.
//! - Shutdown MWF.

use serial_test::serial;

use dev_support::simulation_worker::{SimulationBehavior, SimulationWorker, SimulationWorkerConfig};
use mwf::*;

// MWF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_worker_current_context() {
    MWF::init().unwrap();
    MWF::register_worker_type::<SimulationWorker>().unwrap();

    // Context getters must fail outside an MWF worker task.
    assert!(matches!(MWFWrk::get_current_worker_id(), Err(MWFError::Worker_CurrentContextUnavailable)));
    assert!(matches!(MWFWrk::get_current_parent_id(), Err(MWFError::Worker_CurrentContextUnavailable)));

    // workflow_end() must fail outside an MWF worker task.
    assert!(matches!(MWFWrk::workflow_end(Ok(())), Err(MWFError::Worker_CurrentContextUnavailable)));

    // Create two distinct external parent entities.
    let parent_a_id = EntityID::new_external("PARENT_A");
    let parent_b_id = EntityID::new_external("PARENT_B");

    // Spawn two workers with different parents.
    let worker_a_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &parent_a_id).unwrap();
    let worker_b_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &parent_b_id).unwrap();

    // Configure each worker to validate its context when processing a message.
    let configuration_a =
        SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::Message_CheckMyContext { expected_worker_id: worker_a_id.clone(), expected_parent_id: parent_a_id.clone() }, wait_children_close: false, wait_children_close_timeout_ms: None };

    let configuration_b =
        SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::Message_CheckMyContext { expected_worker_id: worker_b_id.clone(), expected_parent_id: parent_b_id.clone() }, wait_children_close: false, wait_children_close_timeout_ms: None };

    // Initialize both workers.
    MWFWrk::init_worker(&worker_a_id, &configuration_a).await.unwrap();
    MWFWrk::init_worker(&worker_b_id, &configuration_b).await.unwrap();

    let worker_ids = vec![worker_a_id.clone(), worker_b_id.clone()];
    MWFWrk::wait_worker_list_init(&worker_ids, Some(1000)).await.unwrap();

    // Send a message to each worker and wait for its context validation.
    let message_id = MessageID::new("CHECK_CONTEXT");

    let worker_a_result = MWFWrk::send_message_and_wait(&parent_a_id, &worker_a_id, &message_id, MessageWaitMode::WaitThisMessage, &(), Some(1000)).await;

    let worker_b_result = MWFWrk::send_message_and_wait(&parent_b_id, &worker_b_id, &message_id, MessageWaitMode::WaitThisMessage, &(), Some(1000)).await;

    // Shutdown both workers before asserting message results,
    // so the framework can still be closed if a context check fails.
    MWFWrk::close_worker(&worker_a_id, &()).await.unwrap();
    MWFWrk::close_worker(&worker_b_id, &()).await.unwrap();

    MWFWrk::wait_worker_list_close(&worker_ids, Some(1000)).await.unwrap();

    MWF::close().unwrap();

    // Propagate any context validation error to the test.
    worker_a_result.unwrap();
    worker_b_result.unwrap();
}
