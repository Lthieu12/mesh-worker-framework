//! Validate MMF timeout and recovery mechanisms.
//!
//! Scenario:
//! - Simulate a worker that takes too long to initialize.
//! - Verify wait_worker_init() timeout detection.
//! - Simulate a worker that takes too long to shutdown.
//! - Verify wait_worker_close() timeout detection.
//! - Recover using kill_worker().
//! - Shutdown MMF cleanly.
//!
//! This test validates timeout handling and the ability of MMF
//! to recover from unresponsive workers.

use bytes::Bytes;
use serial_test::serial;

use dev_support::simulation_worker::{SimulationBehavior, SimulationWorker, SimulationWorkerConfig};
use mesh_worker_framework::*;

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_worker_timeout_init() {
    MMF::init().unwrap();
    MMF::register_worker_type::<SimulationWorker>().unwrap();

    let worker_id = MMFWrk::spawn_worker(SimulationWorker::worker_type_id(), &EntityID::new_external("MAIN")).unwrap();

    // Very long init
    let configuration = SimulationWorkerConfig { init_delay_ms: 5_000, close_delay_ms: 0, simulation_behavior: SimulationBehavior::DoNothing, wait_children_close: false, wait_children_close_timeout_ms: None };
    let lifecycle_data = Bytes::from(bincode::serialize(&configuration).unwrap());

    MMFWrk::init_worker(&worker_id, lifecycle_data).await.unwrap();

    assert!(matches!(MMFWrk::wait_worker_init(&worker_id, Some(100)).await, Err(MMFError::Operation_Timeout)));

    MMFWrk::abort_worker(&worker_id).unwrap();
    MMF::close().unwrap();
}

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_worker_timeout_close() {
    MMF::init().unwrap();
    MMF::register_worker_type::<SimulationWorker>().unwrap();

    let worker_id = MMFWrk::spawn_worker(SimulationWorker::worker_type_id(), &EntityID::new_external("MAIN")).unwrap();

    // Very long close
    let configuration = SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 5_000, simulation_behavior: SimulationBehavior::DoNothing, wait_children_close: false, wait_children_close_timeout_ms: None };
    let lifecycle_data = Bytes::from(bincode::serialize(&configuration).unwrap());

    MMFWrk::init_worker(&worker_id, lifecycle_data).await.unwrap();

    MMFWrk::wait_worker_init(&worker_id, Some(1000)).await.unwrap();

    MMFWrk::close_worker(&worker_id, Bytes::new()).await.unwrap();

    assert!(matches!(MMFWrk::wait_worker_close(&worker_id, Some(100)).await, Err(MMFError::Operation_Timeout)));

    MMFWrk::abort_worker(&worker_id).unwrap();
    MMF::close().unwrap();
}
