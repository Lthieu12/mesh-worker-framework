//! Validate worker registration and lifecycle failure handling.
//!
//! These tests verify that:
//! - worker types must be registered before spawning.
//! - Initialization failures are correctly propagated.
//! - Shutdown failures are correctly propagated.

use serial_test::serial;

use dev_support::simulation_worker::{SimulationBehavior, SimulationWorker, SimulationWorkerConfig};
use mwf::*;

// MWF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_worker_init_failed() {
    MWF::init().unwrap();

    let result = MWFWrk::spawn_worker(&"SimulationWorker", &EntityID::new_external("MAIN"));

    assert!(matches!(result, Err(MWFError::WorkerType_NotRegistered)));

    MWF::register_worker_type::<SimulationWorker>().unwrap();

    let worker_id = MWFWrk::spawn_worker(&"SimulationWorker", &EntityID::new_external("MAIN")).unwrap();

    let configuration = SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::Init_Fail, wait_children_close: false, wait_children_close_timeout_ms: None };

    MWFWrk::init_worker(&worker_id, &configuration).await.unwrap();

    let worker_id = MWFWrk::wait_worker_list_init(&vec![worker_id.clone()], Some(1000)).await;
    println!("Result: {:?}", worker_id);
    assert!(matches!(worker_id, Err(MWFError::Worker_InitializationFailed)));

    MWFWrk::abort_children(&EntityID::new_external("MAIN")).unwrap();

    MWF::close().unwrap();
}

// MWF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_worker_init_invalid_data_type() {
    MWF::init().unwrap();

    MWF::register_worker_type::<SimulationWorker>().unwrap();

    let worker_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &EntityID::new_external("MAIN")).unwrap();

    // Intentionally send the wrong initialization data type.
    MWFWrk::init_worker(&worker_id, &()).await.unwrap();

    let result = MWFWrk::wait_worker_list_init(&vec![worker_id.clone()], Some(1000)).await;

    assert!(matches!(result, Err(MWFError::Worker_InitializationFailed)));

    // Verify the root cause reported by MWF.
    let error_list = MWFErr::wait_error(None).await.unwrap().expect("Expected error events");

    let error_event = error_list.error_event_list.last().unwrap();
    println!("error_event: {:?}", error_event);

    assert!(error_event.error_description.contains("SimulationWorkerConfig"));
    assert!(error_event.error_description.contains("()"));

    MWFWrk::abort_children(&EntityID::new_external("MAIN")).unwrap();

    MWF::close().unwrap();
}

// MWF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_worker_close_failed() {
    MWF::init().unwrap();

    MWF::register_worker_type::<SimulationWorker>().unwrap();

    let worker_id = MWFWrk::spawn_worker(&"SimulationWorker", &EntityID::new_external("MAIN")).unwrap();

    let configuration = SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::Close_Fail, wait_children_close: false, wait_children_close_timeout_ms: None };

    MWFWrk::init_worker(&worker_id, &configuration).await.unwrap();

    MWFWrk::wait_worker_list_init(&vec![worker_id.clone()], Some(1000)).await.unwrap();

    MWFWrk::close_worker(&worker_id, &()).await.unwrap();

    let result = MWFWrk::wait_worker_list_close(&vec![worker_id.clone()], Some(1000)).await;

    println!("Result: {:?}", result);
    assert!(matches!(result, Err(MWFError::Worker_CloseFailed)));

    MWFWrk::abort_children(&EntityID::new_external("MAIN")).unwrap();

    MWF::close().unwrap();
}
