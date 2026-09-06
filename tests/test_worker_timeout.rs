//! Validate MWF timeout and recovery mechanisms.
//!
//! Scenario:
//! - Simulate a worker that takes too long to initialize.
//! - Verify wait_worker_init() timeout detection.
//! - Simulate a worker that takes too long to shutdown.
//! - Verify wait_worker_close() timeout detection.
//! - Recover using kill_worker().
//! - Shutdown MWF cleanly.
//!
//! This test validates timeout handling and the ability of MWF
//! to recover from unresponsive workers.

use serial_test::serial;

use dev_support::simulation_worker::{SimulationBehavior, SimulationWorker, SimulationWorkerConfig};
use mwf::*;

// MWF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_worker_timeout_init() {
    MWF::init().unwrap();
    MWF::register_worker_type::<SimulationWorker>().unwrap();

    let worker_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &EntityID::new_external("MAIN")).unwrap();

    // Very long init
    let configuration = SimulationWorkerConfig { init_delay_ms: 5_000, close_delay_ms: 0, simulation_behavior: SimulationBehavior::DoNothing, wait_children_close: false, wait_children_close_timeout_ms: None };

    MWFWrk::init_worker(&worker_id, &configuration).await.unwrap();

    assert!(matches!(MWFWrk::wait_worker_init(&worker_id, Some(100)).await, Err(MWFError::Operation_Timeout)));

    MWFWrk::abort_worker(&worker_id).unwrap();
    MWF::close().unwrap();
}

// MWF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_worker_timeout_close() {
    MWF::init().unwrap();
    MWF::register_worker_type::<SimulationWorker>().unwrap();

    let worker_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &EntityID::new_external("MAIN")).unwrap();

    // Very long close
    let configuration = SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 5_000, simulation_behavior: SimulationBehavior::DoNothing, wait_children_close: false, wait_children_close_timeout_ms: None };

    MWFWrk::init_worker(&worker_id, &configuration).await.unwrap();

    MWFWrk::wait_worker_init(&worker_id, Some(1000)).await.unwrap();

    MWFWrk::close_worker(&worker_id, &()).await.unwrap();

    assert!(matches!(MWFWrk::wait_worker_close(&worker_id, Some(100)).await, Err(MWFError::Operation_Timeout)));

    MWFWrk::abort_worker(&worker_id).unwrap();
    MWF::close().unwrap();
}
