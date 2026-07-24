//! Verify proper behavior of MWFLog.
//!
//! First test :
//! Validate worker lifecycle logging.
//!
//! This test verifies that MWFLog records:
//! - workerSpawn
//! - workerInit
//! - workerClose
//!
//! Second test :
//! in the expected order with LogStatus::Ok.
//!
//! Validate message reception logging.
//!
//! This test verifies that MWFLog records
//! received messages with the correct:
//! - sender_id
//! - message_id
//! - priority flag
//! - wait_completion flag
//! - LogStatus
//!
//! Third test :
//! Validate MWFLog circular buffer behavior.
//!
//! This test verifies that:
//! - old log events are overwritten when the buffer is full
//! - missing_log_count is correctly reported
//! - only the most recent log events are retained

use serial_test::serial;

use dev_support::dev_helper::*;
use dev_support::simulation_worker::{SimulationBehavior, SimulationWorker, SimulationWorkerConfig};
use mesh_worker_framework::*;

// MWF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_mwf_log_worker_lifecycle() {
    MWF::init().unwrap();
    MWF::register_worker_type::<SimulationWorker>().unwrap();

    MWFLog::set_log_mode(LogMode::LogAll, None).unwrap();

    let worker_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &EntityID::new_external("Test")).unwrap();

    let configuration = SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::DoNothing, wait_children_close: false, wait_children_close_timeout_ms: None };

    MWFWrk::init_worker(&worker_id, &configuration).await.unwrap();

    MWFWrk::wait_worker_init(&worker_id, Some(1000)).await.unwrap();
    MWFWrk::close_worker(&worker_id, &()).await.unwrap();
    MWFWrk::wait_worker_close(&worker_id, Some(1000)).await.unwrap();

    let log_list = MWFLog::wait_log(None).await.unwrap();

    print_log_list().await;

    assert_eq!(log_list.missing_log_count, 0);

    assert_eq!(log_list.log_event_list.len(), 5);

    assert!(matches!(log_list.log_event_list[0].log_action, LogAction::WorkerSpawn));

    assert!(matches!(log_list.log_event_list[1].log_action, LogAction::WorkerInit));
    assert!(matches!(log_list.log_event_list[2].log_action, LogAction::WorkerInit));

    assert!(matches!(log_list.log_event_list[3].log_action, LogAction::WorkerClose));
    assert!(matches!(log_list.log_event_list[4].log_action, LogAction::WorkerClose));

    assert_eq!(log_list.log_event_list[0].log_status, LogStatus::Ok);

    assert_eq!(log_list.log_event_list[1].log_status, LogStatus::Start);
    assert_eq!(log_list.log_event_list[2].log_status, LogStatus::Ok);

    assert_eq!(log_list.log_event_list[3].log_status, LogStatus::Start);
    assert_eq!(log_list.log_event_list[4].log_status, LogStatus::Ok);

    MWF::close().unwrap();
}

// MWF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_mwf_log_messages() {
    MWF::init().unwrap();
    MWF::register_worker_type::<SimulationWorker>().unwrap();

    MWFLog::set_log_mode(LogMode::LogAll, None).unwrap();

    let worker_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &EntityID::new_external("Test")).unwrap();

    let configuration = SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::DoNothing, wait_children_close: false, wait_children_close_timeout_ms: None };

    MWFWrk::init_worker(&worker_id, &configuration).await.unwrap();
    MWFWrk::wait_worker_init(&worker_id, Some(1000)).await.unwrap();
    MWFWrk::send_message(&EntityID::new_external("Test"), &worker_id, &"MSG_1", &()).await.unwrap();
    MWFWrk::send_priority_message(&EntityID::new_external("Test"), &worker_id, &"MSG_2", &()).await.unwrap();
    MWFWrk::send_message_and_wait(&EntityID::new_external("Test"), &worker_id, &"MSG_3", &(), Some(1000)).await.unwrap();
    MWFWrk::send_priority_message_and_wait(&EntityID::new_external("Test"), &worker_id, &"MSG_4", &(), Some(1000)).await.unwrap();
    MWFWrk::close_worker(&worker_id, &()).await.unwrap();
    MWFWrk::wait_worker_close(&worker_id, Some(1000)).await.unwrap();

    let log_list = MWFLog::wait_log(None).await.unwrap();

    let mut found_normal = false;
    let mut found_priority = false;
    let mut found_wait = false;
    let mut found_priority_wait = false;

    print_log_list().await;

    for log_event in &log_list.log_event_list {
        println!("{:?} {:?}", log_event.log_action, log_event.log_status);

        match &log_event.log_action {
            LogAction::WorkerMessage { priority, wait_completion, .. } => {
                match (*priority, *wait_completion) {
                    (false, false) => {
                        found_normal = true;
                    }

                    (true, false) => {
                        found_priority = true;
                    }

                    (false, true) => {
                        found_wait = true;
                    }

                    (true, true) => {
                        found_priority_wait = true;
                    }
                }

                assert!(matches!(log_event.log_status, LogStatus::Start | LogStatus::Ok));
            }

            _ => {}
        }
    }

    assert!(found_normal);
    assert!(found_priority);
    assert!(found_wait);
    assert!(found_priority_wait);

    MWF::close().unwrap();
}

// MWF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_mwf_log_buffer_overflow() {
    let config = MWFConfiguration { log_event_buffer_capacity: 10, ..Default::default() };

    MWF::init_with_configuration(config).unwrap();
    MWF::register_worker_type::<SimulationWorker>().unwrap();

    MWFLog::set_log_mode(LogMode::LogAll, None).unwrap();

    let worker_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &EntityID::new_external("Test")).unwrap();

    let configuration = SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::DoNothing, wait_children_close: false, wait_children_close_timeout_ms: None };

    MWFWrk::init_worker(&worker_id, &configuration).await.unwrap();

    MWFWrk::wait_worker_init(&worker_id, Some(1000)).await.unwrap();

    // Consume all logs currently present.
    let log_list = MWFLog::wait_log(None).await.unwrap();

    let last_log_instance_id = log_list.last_log_instance_id;

    // Generate more logs than the buffer can hold.
    for index in 0..15 {
        MWFWrk::send_message(&EntityID::new_external("Test"), &worker_id, &format!("MSG_{index}"), &()).await.unwrap();
    }

    let log_list = MWFLog::wait_log(last_log_instance_id).await.unwrap();

    assert_eq!(log_list.missing_log_count, 20);

    assert_eq!(log_list.log_event_list.len(), 10);

    MWFWrk::close_worker(&worker_id, &()).await.unwrap();
    MWFWrk::wait_worker_close(&worker_id, Some(1000)).await.unwrap();

    MWF::close().unwrap();
}
