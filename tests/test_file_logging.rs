//! Verifies persistent file logging, including file creation,
//! log writing and automatic shutdown when the maximum file
//! size is reached.

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
}
