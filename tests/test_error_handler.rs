//! Verify that MMFErr records callback failures.
//!
//! The first test generates failures from worker lifecycle
//! callbacks (on_init and on_close) as well as business
//! message processing (on_message) and verifies that
//! the resulting error events can be retrieved through
//! MMFErr.
//!
//! The second test verify that MMFErr correctly reports overwritten
//! error events when the circular buffer capacity is
//! exceeded.

use anyhow::{Context, Result};
use bytes::Bytes;
use serial_test::serial;

use crate::mmf::MMFConfiguration;
use dev_support::dev_helper::*;
use dev_support::simulation_worker::{SimulationBehavior, SimulationWorker, SimulationWorkerConfig};
use mesh_worker_framework::*;

fn generate_application_error() -> Result<()> {
    Err(anyhow::anyhow!("config.toml not found")).context("Failed to load configuration")
}

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_mmf_err_worker_failures() {
    MMF::init().unwrap();
    MMF::register_worker_type::<SimulationWorker>().unwrap();

    // Generale application error (external error, not related to a worker).
    let application_error = generate_application_error().unwrap_err();

    MMFErr::add_error(&EntityID::new_external("Application"), &application_error).unwrap();

    // Simulation worker with error on message
    let worker_a_id = MMFWrk::spawn_worker(SimulationWorker::worker_type_id(), &EntityID::new_external("Test")).unwrap();

    let configuration = SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::Message_Fail, wait_children_close: false, wait_children_close_timeout_ms: None };
    let lifecycle_data = Bytes::from(bincode::serialize(&configuration).unwrap());

    MMFWrk::init_worker(&worker_a_id, lifecycle_data).await.unwrap();
    MMFWrk::wait_worker_init(&worker_a_id, Some(1000)).await.unwrap();

    // Generate multiple message failures.
    for _ in 0..3 {
        MMFWrk::send_message(&EntityID::new_external("Test"), &worker_a_id, "TEST", Bytes::new()).await.unwrap();
    }

    // Simulation worker with error on init
    let worker_b_id = MMFWrk::spawn_worker(SimulationWorker::worker_type_id(), &EntityID::new_external("Test")).unwrap();

    let configuration = SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::Init_Fail, wait_children_close: false, wait_children_close_timeout_ms: None };
    let lifecycle_data = Bytes::from(bincode::serialize(&configuration).unwrap());

    MMFWrk::init_worker(&worker_b_id, lifecycle_data).await.unwrap();
    let result = MMFWrk::wait_worker_init(&worker_b_id, Some(1000)).await;
    assert!(matches!(result, Err(MMFError::Worker_InitializationFailed)));

    // Simulation worker with error on close
    let worker_c_id = MMFWrk::spawn_worker(SimulationWorker::worker_type_id(), &EntityID::new_external("Test")).unwrap();

    let configuration = SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::Close_Fail, wait_children_close: false, wait_children_close_timeout_ms: None };
    let lifecycle_data = Bytes::from(bincode::serialize(&configuration).unwrap());

    MMFWrk::init_worker(&worker_c_id, lifecycle_data).await.unwrap();
    MMFWrk::wait_worker_init(&worker_c_id, Some(1000)).await.unwrap();
    MMFWrk::close_worker(&worker_c_id, Bytes::new()).await.unwrap();

    let result = MMFWrk::wait_worker_close(&worker_c_id, Some(1000)).await;
    assert!(matches!(result, Err(MMFError::Worker_CloseFailed)));

    let error_list = MMFErr::wait_error(None).await.unwrap();

    print_error_list().await;

    assert_eq!(error_list.missing_error_count, 0);

    assert_eq!(error_list.error_event_list.len(), 6);

    assert!(error_list.first_error_instance_id.is_some());

    assert!(error_list.last_error_instance_id.is_some());

    let error_messages: Vec<&str> = error_list.error_event_list.iter().map(|error_event| error_event.error_description.as_str()).collect();

    assert_eq!(error_messages.iter().filter(|message| **message == "Simulation message failure").count(), 3);

    assert_eq!(error_messages.iter().filter(|message| **message == "Simulation init failure").count(), 1);

    assert_eq!(error_messages.iter().filter(|message| **message == "Simulation close failure").count(), 1);

    let application_error = error_list.error_event_list.iter().find(|error_event| error_event.entity_id == EntityID::External("Application".into())).unwrap();

    assert_eq!(application_error.error_description, "Failed to load configuration");

    assert_eq!(application_error.error_context_list[0], "config.toml not found");

    for error_event in &error_list.error_event_list {
        match &error_event.entity_id {
            EntityID::External(entity_id) => {
                assert_eq!(entity_id.as_str(), "Application");
            }

            EntityID::Worker(_) => {}
        }

        assert!(!error_event.timestamp_rfc3339.is_empty());
    }

    // Verify ordering of ErrorInstanceID.
    for window in error_list.error_event_list.windows(2) {
        assert!(window[0].error_instance_id < window[1].error_instance_id);
    }

    MMFWrk::abort_worker(&worker_a_id).unwrap();
    MMFWrk::abort_worker(&worker_b_id).unwrap();
    MMFWrk::abort_worker(&worker_c_id).unwrap();

    MMF::close().unwrap();
}

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_mmf_err_buffer_overflow() {
    let mut configuration = MMFConfiguration::default();

    configuration.error_event_buffer_capacity = 10;
    configuration.log_event_buffer_capacity = 10;

    MMF::init_with_configuration(configuration).unwrap();
    MMF::register_worker_type::<SimulationWorker>().unwrap();

    let worker_id = MMFWrk::spawn_worker(SimulationWorker::worker_type_id(), &EntityID::new_external("Test")).unwrap();

    let configuration = SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::Message_Fail, wait_children_close: false, wait_children_close_timeout_ms: None };
    let lifecycle_data = Bytes::from(bincode::serialize(&configuration).unwrap());

    MMFWrk::init_worker(&worker_id, lifecycle_data).await.unwrap();

    MMFWrk::wait_worker_init(&worker_id, Some(1000)).await.unwrap();

    // Generate more errors than the buffer can hold.
    for _ in 0..15 {
        let _ = MMFWrk::send_message_and_wait(&EntityID::new_external("Test"), &worker_id, "TEST", Bytes::new(), Some(1000)).await;
    }

    let error_list = MMFErr::wait_error(Some(0)).await.unwrap();

    assert_eq!(error_list.missing_error_count, 5);

    assert_eq!(error_list.first_error_instance_id, Some(6));

    assert_eq!(error_list.last_error_instance_id, Some(15));

    assert_eq!(error_list.error_event_list.len(), 10);

    MMFWrk::abort_children(&EntityID::new_external("Test")).unwrap();

    MMF::close().unwrap();
}
