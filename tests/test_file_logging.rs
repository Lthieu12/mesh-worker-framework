//! Verifies that reaching the maximum log file size disables file logging
//! without interrupting in-memory logging.
//!
//! Test scenario:
//! 1. Initialize MWF with a small maximum log file size.
//! 2. Enable logging for all workers and file logging.
//! 3. Spawn and initialize a simulation worker.
//! 4. Send enough messages to reach the maximum log file size.
//! 5. Close the worker and verify that its close event is still available
//!    in the in-memory log buffer.
//! 6. Close MWF and read the generated log file.
//! 7. Verify that the file contains the expected events and the size-limit
//!    notification, but no events generated after file logging was disabled.
use serial_test::serial;

use dev_support::simulation_worker::{SimulationBehavior, SimulationWorker, SimulationWorkerConfig};
use mwf::*;

// MWF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_file_logging() {
    use std::fs;

    // Configure MWF.
    let configuration = MWFConfiguration { log_file_max_size_bytes: 4096, ..Default::default() };

    MWF::init_with_configuration(configuration).unwrap();

    let log_file_path = MWFLog::get_log_file_path().unwrap().unwrap();

    MWF::register_worker_type::<SimulationWorker>().unwrap();

    MWFLog::set_log_mode(LogMode::LogAll, None).unwrap();
    MWFLog::set_log_file_enabled(true).unwrap();

    let simulation_configuration = SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::DoNothing, wait_children_close: false, wait_children_close_timeout_ms: None };

    let worker_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &EntityID::new_external("Test")).unwrap();

    MWFWrk::init_worker(&worker_id, &simulation_configuration).await.unwrap();

    MWFWrk::wait_worker_init(&worker_id, None).await.unwrap();

    // Generate enough log events to fill the log file.
    for index in 0..100 {
        MWFWrk::send_message(&EntityID::new_external("Test"), &worker_id, &format!("Message{index}"), &()).await.unwrap();
    }

    MWFWrk::close_worker(&worker_id, &()).await.unwrap();

    MWFWrk::wait_worker_close(&worker_id, None).await.unwrap();

    // Verify that worker close events remain available in memory
    // after file logging has been disabled.
    let log_list = MWFLog::wait_log(None).await.unwrap().expect("Expected log events");

    assert!(log_list.log_event_list.iter().any(|log_event| { matches!(&log_event.log_action, LogAction::WorkerClose) && log_event.log_status == LogStatus::Ok }));
    assert!(matches!(MWFLog::set_log_file_enabled(true), Err(MWFError::Logging_LogFileAccessFailed(_))));

    MWF::close().unwrap();

    println!("{:?}", std::env::current_dir().unwrap());
    println!("{:?}", std::env::current_exe().unwrap());

    // Read the generated log file.
    let log_file = fs::read_to_string(log_file_path).unwrap();

    // Validate the file content.
    assert!(log_file.contains("Mesh Worker Framework Log File"));
    assert!(log_file.contains("LogID,Time,WorkerID"));
    assert!(log_file.contains("SPAWN"));
    assert!(log_file.contains("INIT"));
    assert!(log_file.contains("MESSAGE"));
    assert!(log_file.contains("Maximum log file size reached."));
    assert!(log_file.contains("File logging disabled."));
    assert!(!log_file.contains(",CLOSE,"));
}
