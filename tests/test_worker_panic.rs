//! Verify that MMF recovers from a worker panic.
//!
//! The panicked worker must be removed from the Registry,
//! the panic must be reported, and the runtime must remain
//! operational.

use bytes::Bytes;
use serial_test::serial;

use dev_support::simulation_worker::{SimulationBehavior, SimulationWorker, SimulationWorkerConfig};
use mesh_worker_framework::*;

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_worker_message_panic() {
    MMF::init().unwrap();
    MMF::register_worker_type::<SimulationWorker>().unwrap();

    MMFLog::set_log_mode(LogMode::LogAll, None).unwrap();

    let worker_id = MMFWrk::spawn_worker(SimulationWorker::worker_type_id(), &EntityID::new_external("MAIN")).unwrap();

    let configuration = SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::Message_Panic, wait_children_close: false, wait_children_close_timeout_ms: None };
    let lifecycle_data = Bytes::from(bincode::serialize(&configuration).unwrap());

    MMFWrk::init_worker(&worker_id, lifecycle_data).await.unwrap();

    MMFWrk::wait_worker_init(&worker_id, Some(1000)).await.unwrap();

    // Trigger the panic.
    MMFWrk::send_message(&EntityID::new_external("MAIN"), &worker_id, "TEST", bytes::Bytes::new()).await.unwrap();

    // Wait until MMF has removed the worker.
    _ = MMFWrk::wait_worker_close(&worker_id, Some(1000)).await;

    // The worker must no longer exist.
    assert!(MMFWrk::get_worker_status(&worker_id).is_err());

    // Wait for the panic to be recorded.
    let error_list = MMFErr::wait_error(None).await.unwrap();

    assert_eq!(error_list.error_event_list.len(), 1);

    assert!(error_list.error_event_list[0].error_description.contains("Simulation message panic"));

    // One panic event must be present in the log buffer.
    let log_list = MMFLog::wait_log(None).await.unwrap();

    assert!(log_list.log_event_list.iter().any(|log| matches!(log.log_action, LogAction::WorkerPanic)));

    MMF::close().unwrap();
}
