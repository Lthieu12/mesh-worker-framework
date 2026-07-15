//! Integration tests for MMF message handling.
//!
//! Validates:
//! - Normal message FIFO ordering.
//! - Priority message ordering.
//! - send_message().
//! - send_priority_message().
//! - send_message_and_wait().
//! - send_unique_message().

use bytes::Bytes;
use serial_test::serial;
use std::time::Duration;

use mesh_module_framework::*;

use dev_support::dev_helper::*;
use dev_support::simulation_module::{SimulationBehavior, SimulationModule, SimulationModuleConfig};

const MESSAGE_NORMAL_1: &str = "NORMAL_1";
const MESSAGE_NORMAL_2: &str = "NORMAL_2";
const MESSAGE_PRIORITY_1: &str = "PRIORITY_1";
const MESSAGE_PRIORITY_2: &str = "PRIORITY_2";

const MESSAGE_CHAIN: &str = "CHAIN";

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_message_priority_order() {
    MMF::init().unwrap();

    MMFLog::set_log_mode(LogMode::LogAll, None).unwrap();

    MMF::register_module_type::<SimulationModule>().unwrap();

    let module_a_id = MMFMod::spawn_module(SimulationModule::module_type_id(), &EntityID::External("MAIN".into())).unwrap();

    let configuration = SimulationModuleConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::Message_Wait { message_delay_ms: 100 }, wait_children_close: false, wait_children_close_timeout_ms: None };
    let lifecycle_data = Bytes::from(bincode::serialize(&configuration).unwrap());

    MMFMod::init_module(&module_a_id, lifecycle_data).await.unwrap();

    MMFMod::wait_module_init(&module_a_id, Some(1000)).await.unwrap();

    // First normal message starts processing.
    MMFMod::send_message(&EntityID::External("MAIN".into()), &module_a_id, &MESSAGE_NORMAL_1.into(), Bytes::new()).await.unwrap();

    // Ensure the module is busy before sending the remaining messages.
    tokio::time::sleep(Duration::from_millis(20)).await;

    MMFMod::send_message(&EntityID::External("MAIN".into()), &module_a_id, &MESSAGE_NORMAL_2.into(), Bytes::new()).await.unwrap();

    MMFMod::send_priority_message(&EntityID::External("MAIN".into()), &module_a_id, &MESSAGE_PRIORITY_1.into(), Bytes::new()).await.unwrap();

    MMFMod::send_priority_message(&EntityID::External("MAIN".into()), &module_a_id, &MESSAGE_PRIORITY_2.into(), Bytes::new()).await.unwrap();

    // Wait until every message has been processed.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let log_list = MMFLog::wait_log(None).await.unwrap();

    let messages = filter_messages(&log_list, &module_a_id);

    print_log_list().await;

    assert_eq!(messages.len(), 8);

    // NORMAL_1
    assert_eq!(messages[0].message_id, MESSAGE_NORMAL_1);
    assert_eq!(messages[0].log_status, LogStatus::Start);
    assert!(!messages[0].priority);
    assert!(!messages[0].wait_completion);

    assert_eq!(messages[1].message_id, MESSAGE_NORMAL_1);
    assert_eq!(messages[1].log_status, LogStatus::Ok);
    assert!(!messages[1].priority);
    assert!(!messages[1].wait_completion);

    // PRIORITY_2
    assert_eq!(messages[2].message_id, MESSAGE_PRIORITY_2);
    assert_eq!(messages[2].log_status, LogStatus::Start);
    assert!(messages[2].priority);
    assert!(!messages[2].wait_completion);

    assert_eq!(messages[3].message_id, MESSAGE_PRIORITY_2);
    assert_eq!(messages[3].log_status, LogStatus::Ok);
    assert!(messages[3].priority);
    assert!(!messages[3].wait_completion);

    // PRIORITY_1
    assert_eq!(messages[4].message_id, MESSAGE_PRIORITY_1);
    assert_eq!(messages[4].log_status, LogStatus::Start);
    assert!(messages[4].priority);
    assert!(!messages[4].wait_completion);

    assert_eq!(messages[5].message_id, MESSAGE_PRIORITY_1);
    assert_eq!(messages[5].log_status, LogStatus::Ok);
    assert!(messages[5].priority);
    assert!(!messages[5].wait_completion);

    // NORMAL_2
    assert_eq!(messages[6].message_id, MESSAGE_NORMAL_2);
    assert_eq!(messages[6].log_status, LogStatus::Start);
    assert!(!messages[6].priority);
    assert!(!messages[6].wait_completion);

    assert_eq!(messages[7].message_id, MESSAGE_NORMAL_2);
    assert_eq!(messages[7].log_status, LogStatus::Ok);
    assert!(!messages[7].priority);
    assert!(!messages[7].wait_completion);

    MMFMod::close_module(&module_a_id, Bytes::new()).await.unwrap();

    MMFMod::wait_children_close(&EntityID::External("MAIN".into()), Some(1000)).await.unwrap();

    MMF::close().unwrap();
}

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_send_message_and_wait() {
    MMF::init().unwrap();

    MMFLog::set_log_mode(LogMode::LogAll, None).unwrap();

    MMF::register_module_type::<SimulationModule>().unwrap();

    //
    // Spawn module B.
    //
    let module_b_id = MMFMod::spawn_module(SimulationModule::module_type_id(), &EntityID::External("MAIN".into())).unwrap();

    let configuration = SimulationModuleConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::Message_Wait { message_delay_ms: 100 }, wait_children_close: false, wait_children_close_timeout_ms: None };

    let lifecycle_data = Bytes::from(bincode::serialize(&configuration).unwrap());

    MMFMod::init_module(&module_b_id, lifecycle_data).await.unwrap();

    MMFMod::wait_module_init(&module_b_id, Some(1000)).await.unwrap();

    //
    // Spawn module A.
    //
    let module_a_id = MMFMod::spawn_module(SimulationModule::module_type_id(), &EntityID::External("MAIN".into())).unwrap();

    let configuration = SimulationModuleConfig {
        init_delay_ms: 0,
        close_delay_ms: 0,
        simulation_behavior: SimulationBehavior::Message_SendAndWait { target_module_id: module_b_id.clone(), message_id: MESSAGE_CHAIN.into(), timeout_ms: Some(1000), message_delay_ms: 0 },
        wait_children_close: false,
        wait_children_close_timeout_ms: None,
    };

    let lifecycle_data = Bytes::from(bincode::serialize(&configuration).unwrap());

    MMFMod::init_module(&module_a_id, lifecycle_data).await.unwrap();

    MMFMod::wait_module_init(&module_a_id, Some(1000)).await.unwrap();

    //
    // Trigger the chain.
    //
    MMFMod::send_message(&EntityID::External("MAIN".into()), &module_a_id, &MESSAGE_CHAIN.into(), Bytes::new()).await.unwrap();

    //
    // Wait until processing completes.
    //
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;

    let log_list = MMFLog::wait_log(None).await.unwrap();

    let messages_a = filter_messages(&log_list, &module_a_id);
    let messages_b = filter_messages(&log_list, &module_b_id);

    print_log_list().await;

    //
    // Module A.
    //
    assert_eq!(messages_a.len(), 2);

    assert_eq!(messages_a[0].message_id, MESSAGE_CHAIN);
    assert_eq!(messages_a[0].log_status, LogStatus::Start);
    assert!(!messages_a[0].priority);
    assert!(!messages_a[0].wait_completion);

    assert_eq!(messages_a[1].message_id, MESSAGE_CHAIN);
    assert_eq!(messages_a[1].log_status, LogStatus::Ok);

    //
    // Module B.
    //
    assert_eq!(messages_b.len(), 2);

    assert_eq!(messages_b[0].message_id, MESSAGE_CHAIN);
    assert_eq!(messages_b[0].log_status, LogStatus::Start);
    assert!(!messages_b[0].priority);
    assert!(messages_b[0].wait_completion);

    assert_eq!(messages_b[0].sender_id, &EntityID::Module(module_a_id.clone()));

    assert_eq!(messages_b[1].message_id, MESSAGE_CHAIN);
    assert_eq!(messages_b[1].log_status, LogStatus::Ok);

    MMFMod::close_module(&module_a_id, Bytes::new()).await.unwrap();
    MMFMod::close_module(&module_b_id, Bytes::new()).await.unwrap();

    MMFMod::wait_children_close(&EntityID::External("MAIN".into()), Some(1000)).await.unwrap();

    MMF::close().unwrap();
}

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_send_unique_message() {
    // Initialize MMF.
    MMF::init().unwrap();

    MMFLog::set_log_mode(LogMode::LogAll, None).unwrap();

    MMF::register_module_type::<SimulationModule>().unwrap();

    // Spawn and initialize the simulation module.
    let module_id = MMFMod::spawn_module(SimulationModule::module_type_id(), &EntityID::External("MAIN".into())).unwrap();

    let configuration = SimulationModuleConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::Message_Wait { message_delay_ms: 100 }, wait_children_close: false, wait_children_close_timeout_ms: None };

    let lifecycle_data = Bytes::from(bincode::serialize(&configuration).unwrap());

    MMFMod::init_module(&module_id, lifecycle_data).await.unwrap();
    MMFMod::wait_module_init(&module_id, Some(1000)).await.unwrap();

    // Send a blocking message to keep the module busy.
    MMFMod::send_message(&EntityID::External("MAIN".into()), &module_id, &MESSAGE_CHAIN.into(), Bytes::new()).await.unwrap();

    // Give the module enough time to start processing
    // the blocking message.
    MMFMod::sleep(Duration::from_millis(20)).await;

    // Send several equivalent unique messages.
    for _ in 0..5 {
        MMFMod::send_unique_message(&EntityID::External("MAIN".into()), &module_id, &MESSAGE_CHAIN.into(), Bytes::new()).await.unwrap();
    }

    // Wait until all messages have been processed.
    MMFMod::sleep(Duration::from_millis(300)).await;

    // Verify that only the first and the last unique
    // messages have been processed.
    let log_list = MMFLog::wait_log(None).await.unwrap();
    print_log_list().await;

    let messages = filter_messages(&log_list, &module_id);

    assert_eq!(messages.len(), 4);

    // Start of the blocking message.
    assert_eq!(messages[0].message_id, &MESSAGE_CHAIN.to_string());
    assert_eq!(messages[0].log_status, LogStatus::Start);

    // End of the blocking message.
    assert_eq!(messages[1].message_id, &MESSAGE_CHAIN.to_string());
    assert_eq!(messages[1].log_status, LogStatus::Ok);

    // Start of the last unique message.
    assert_eq!(messages[2].message_id, &MESSAGE_CHAIN.to_string());
    assert_eq!(messages[2].log_status, LogStatus::Start);

    // End of the last unique message.
    assert_eq!(messages[3].message_id, &MESSAGE_CHAIN.to_string());
    assert_eq!(messages[3].log_status, LogStatus::Ok);

    // The unique message must correspond to the fifth unique
    // message that was sent.
    assert_eq!(*messages[2].message_instance_id, *messages[0].message_instance_id + 5);

    assert_eq!(*messages[3].message_instance_id, *messages[2].message_instance_id);

    MMFMod::close_module(&module_id, Bytes::new()).await.unwrap();
    MMFMod::wait_module_close(&module_id, Some(1000)).await.unwrap();

    MMF::close().unwrap();
}
