//! Integration tests for MWF message handling.
//!
//! Validates:
//! - Normal message FIFO ordering.
//! - Priority message ordering.
//! - send_message().
//! - send_priority_message().
//! - send_message_and_wait().
//! - send_unique_message().

use serial_test::serial;
use std::time::Duration;

use mesh_worker_framework::*;

use dev_support::dev_helper::*;
use dev_support::simulation_worker::{SimulationBehavior, SimulationWorker, SimulationWorkerConfig};

const MESSAGE_NORMAL_1: &str = "NORMAL_1";
const MESSAGE_NORMAL_2: &str = "NORMAL_2";
const MESSAGE_PRIORITY_1: &str = "PRIORITY_1";
const MESSAGE_PRIORITY_2: &str = "PRIORITY_2";

const MESSAGE_CHAIN: &str = "CHAIN";

// MWF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_message_priority_order() {
    MWF::init().unwrap();

    MWFLog::set_log_mode(LogMode::LogAll, None).unwrap();

    MWF::register_worker_type::<SimulationWorker>().unwrap();

    let worker_a_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &EntityID::new_external("MAIN")).unwrap();

    let configuration = SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::Message_Wait { message_delay_ms: 100 }, wait_children_close: false, wait_children_close_timeout_ms: None };

    MWFWrk::init_worker(&worker_a_id, &configuration).await.unwrap();

    MWFWrk::wait_worker_init(&worker_a_id, Some(1000)).await.unwrap();

    // First normal message starts processing.
    MWFWrk::send_message(&EntityID::new_external("MAIN"), &worker_a_id, &MESSAGE_NORMAL_1, &()).await.unwrap();

    // Ensure the worker is busy before sending the remaining messages.
    tokio::time::sleep(Duration::from_millis(20)).await;

    MWFWrk::send_message(&EntityID::new_external("MAIN"), &worker_a_id, &MESSAGE_NORMAL_2, &()).await.unwrap();

    MWFWrk::send_priority_message(&EntityID::new_external("MAIN"), &worker_a_id, &MESSAGE_PRIORITY_1, &()).await.unwrap();

    MWFWrk::send_priority_message(&EntityID::new_external("MAIN"), &worker_a_id, &MESSAGE_PRIORITY_2, &()).await.unwrap();

    // Wait until every message has been processed.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let log_list = MWFLog::wait_log(None).await.unwrap();

    let messages = filter_messages(&log_list, &worker_a_id);

    print_log_list().await;

    assert_eq!(messages.len(), 8);

    // NORMAL_1
    assert_eq!(messages[0].message_id.as_str(), MESSAGE_NORMAL_1);
    assert_eq!(messages[0].log_status, LogStatus::Start);
    assert!(!messages[0].priority);
    assert!(!messages[0].wait_completion);

    assert_eq!(messages[1].message_id.as_str(), MESSAGE_NORMAL_1);
    assert_eq!(messages[1].log_status, LogStatus::Ok);
    assert!(!messages[1].priority);
    assert!(!messages[1].wait_completion);

    // PRIORITY_2
    assert_eq!(messages[2].message_id.as_str(), MESSAGE_PRIORITY_2);
    assert_eq!(messages[2].log_status, LogStatus::Start);
    assert!(messages[2].priority);
    assert!(!messages[2].wait_completion);

    assert_eq!(messages[3].message_id.as_str(), MESSAGE_PRIORITY_2);
    assert_eq!(messages[3].log_status, LogStatus::Ok);
    assert!(messages[3].priority);
    assert!(!messages[3].wait_completion);

    // PRIORITY_1
    assert_eq!(messages[4].message_id.as_str(), MESSAGE_PRIORITY_1);
    assert_eq!(messages[4].log_status, LogStatus::Start);
    assert!(messages[4].priority);
    assert!(!messages[4].wait_completion);

    assert_eq!(messages[5].message_id.as_str(), MESSAGE_PRIORITY_1);
    assert_eq!(messages[5].log_status, LogStatus::Ok);
    assert!(messages[5].priority);
    assert!(!messages[5].wait_completion);

    // NORMAL_2
    assert_eq!(messages[6].message_id.as_str(), MESSAGE_NORMAL_2);
    assert_eq!(messages[6].log_status, LogStatus::Start);
    assert!(!messages[6].priority);
    assert!(!messages[6].wait_completion);

    assert_eq!(messages[7].message_id.as_str(), MESSAGE_NORMAL_2);
    assert_eq!(messages[7].log_status, LogStatus::Ok);
    assert!(!messages[7].priority);
    assert!(!messages[7].wait_completion);

    MWFWrk::close_worker(&worker_a_id, &()).await.unwrap();

    MWFWrk::wait_children_close(&EntityID::new_external("MAIN"), Some(1000)).await.unwrap();

    MWF::close().unwrap();
}

// MWF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_send_message_and_wait() {
    MWF::init().unwrap();

    MWFLog::set_log_mode(LogMode::LogAll, None).unwrap();

    MWF::register_worker_type::<SimulationWorker>().unwrap();

    //
    // Spawn worker B.
    //
    let worker_b_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &EntityID::new_external("MAIN")).unwrap();

    let configuration = SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::Message_Wait { message_delay_ms: 100 }, wait_children_close: false, wait_children_close_timeout_ms: None };

    MWFWrk::init_worker(&worker_b_id, &configuration).await.unwrap();

    MWFWrk::wait_worker_init(&worker_b_id, Some(1000)).await.unwrap();

    //
    // Spawn worker A.
    //
    let worker_a_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &EntityID::new_external("MAIN")).unwrap();

    let configuration = SimulationWorkerConfig {
        init_delay_ms: 0,
        close_delay_ms: 0,
        simulation_behavior: SimulationBehavior::Message_SendAndWait { target_worker_id: worker_b_id.clone(), message_id: MESSAGE_CHAIN.into(), timeout_ms: Some(1000), message_delay_ms: 0 },
        wait_children_close: false,
        wait_children_close_timeout_ms: None,
    };

    MWFWrk::init_worker(&worker_a_id, &configuration).await.unwrap();

    MWFWrk::wait_worker_init(&worker_a_id, Some(1000)).await.unwrap();

    //
    // Trigger the chain.
    //
    MWFWrk::send_message(&EntityID::new_external("MAIN"), &worker_a_id, &MESSAGE_CHAIN, &()).await.unwrap();

    //
    // Wait until processing completes.
    //
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;

    let log_list = MWFLog::wait_log(None).await.unwrap();

    let messages_a = filter_messages(&log_list, &worker_a_id);
    let messages_b = filter_messages(&log_list, &worker_b_id);

    print_log_list().await;

    //
    // worker A.
    //
    assert_eq!(messages_a.len(), 2);

    assert_eq!(messages_a[0].message_id.as_str(), MESSAGE_CHAIN);
    assert_eq!(messages_a[0].log_status, LogStatus::Start);
    assert!(!messages_a[0].priority);
    assert!(!messages_a[0].wait_completion);

    assert_eq!(messages_a[1].message_id.as_str(), MESSAGE_CHAIN);
    assert_eq!(messages_a[1].log_status, LogStatus::Ok);

    //
    // worker B.
    //
    assert_eq!(messages_b.len(), 2);

    assert_eq!(messages_b[0].message_id.as_str(), MESSAGE_CHAIN);
    assert_eq!(messages_b[0].log_status, LogStatus::Start);
    assert!(!messages_b[0].priority);
    assert!(messages_b[0].wait_completion);

    assert_eq!(messages_b[0].sender_id, &EntityID::Worker(worker_a_id.clone()));

    assert_eq!(messages_b[1].message_id.as_str(), MESSAGE_CHAIN);
    assert_eq!(messages_b[1].log_status, LogStatus::Ok);

    MWFWrk::close_worker(&worker_a_id, &()).await.unwrap();
    MWFWrk::close_worker(&worker_b_id, &()).await.unwrap();

    MWFWrk::wait_children_close(&EntityID::new_external("MAIN"), Some(1000)).await.unwrap();

    MWF::close().unwrap();
}

// MWF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_send_unique_message() {
    // Initialize MWF.
    MWF::init().unwrap();

    MWFLog::set_log_mode(LogMode::LogAll, None).unwrap();

    MWF::register_worker_type::<SimulationWorker>().unwrap();

    // Spawn and initialize the simulation worker.
    let worker_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &EntityID::new_external("MAIN")).unwrap();

    let configuration = SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::Message_Wait { message_delay_ms: 100 }, wait_children_close: false, wait_children_close_timeout_ms: None };

    MWFWrk::init_worker(&worker_id, &configuration).await.unwrap();
    MWFWrk::wait_worker_init(&worker_id, Some(1000)).await.unwrap();

    // Send a blocking message to keep the worker busy.
    MWFWrk::send_message(&EntityID::new_external("MAIN"), &worker_id, &MESSAGE_CHAIN, &()).await.unwrap();

    // Give the worker enough time to start processing
    // the blocking message.
    MWFWrk::sleep(Duration::from_millis(20)).await;

    // Send several equivalent unique messages.
    for _ in 0..5 {
        MWFWrk::send_unique_message(&EntityID::new_external("MAIN"), &worker_id, &MESSAGE_CHAIN, &()).await.unwrap();
    }

    // Wait until all messages have been processed.
    MWFWrk::sleep(Duration::from_millis(300)).await;

    // Verify that only the first and the last unique
    // messages have been processed.
    let log_list = MWFLog::wait_log(None).await.unwrap();
    print_log_list().await;

    let messages = filter_messages(&log_list, &worker_id);

    assert_eq!(messages.len(), 4);

    // Start of the blocking message.
    assert_eq!(messages[0].message_id.as_str(), &MESSAGE_CHAIN.to_string());
    assert_eq!(messages[0].log_status, LogStatus::Start);

    // End of the blocking message.
    assert_eq!(messages[1].message_id.as_str(), &MESSAGE_CHAIN.to_string());
    assert_eq!(messages[1].log_status, LogStatus::Ok);

    // Start of the last unique message.
    assert_eq!(messages[2].message_id.as_str(), &MESSAGE_CHAIN.to_string());
    assert_eq!(messages[2].log_status, LogStatus::Start);

    // End of the last unique message.
    assert_eq!(messages[3].message_id.as_str(), &MESSAGE_CHAIN.to_string());
    assert_eq!(messages[3].log_status, LogStatus::Ok);

    // The unique message must correspond to the fifth unique
    // message that was sent.
    assert_eq!(*messages[2].message_instance_id, *messages[0].message_instance_id + 5);

    assert_eq!(*messages[3].message_instance_id, *messages[2].message_instance_id);

    MWFWrk::close_worker(&worker_id, &()).await.unwrap();
    MWFWrk::wait_worker_close(&worker_id, Some(1000)).await.unwrap();

    MWF::close().unwrap();
}
