//! Integration tests for MMF messaging features.
//!
//! Validates:
//! - send_message()
//! - send_priority_message()
//! - send_message_and_wait_reply()
//! - Message ordering guarantees.
//! - Priority message handling.
//! - Chained message processing across multiple modules.

mod common;

use serial_test::serial;
use std::sync::{Arc, Mutex};
use bytes::Bytes;
use std::time::Duration;

use mesh_module_framework::*;
use common::simulation_module::{SimulationModule, SimulationMessageBehavior};
use common::simulation_module_recorder::{MessageRecorderModule, MESSAGE_CHAIN, MESSAGE_NORMAL_1, MESSAGE_NORMAL_2, MESSAGE_PRIORITY_1, MESSAGE_PRIORITY_2};

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_module_message_chain() {

    MMF::init().unwrap();

    let execution_log =
        Arc::new(Mutex::new(Vec::<String>::new()));

    let wait_duration_ms =
        Arc::new(Mutex::new(None));

    let module_a_id: ModuleID;
    let module_b_id: ModuleID;
    let module_c_id: ModuleID;
    let message_id_temp: MessageID = Default::default();

    module_c_id = SimulationModule::spawn(&EntityID::External("MAIN".to_string()), 300, 300, SimulationMessageBehavior::Wait { message_delay_ms: (300) }, false, None).await.unwrap();
    module_b_id = SimulationModule::spawn(&EntityID::External("MAIN".to_string()), 200, 200, SimulationMessageBehavior::SendAndWaitReply { target_module_id: module_c_id.clone(), message_id: message_id_temp, timeout_ms: (Some(500)), message_delay_ms: (200) }, false, None).await.unwrap();
    module_a_id = MessageRecorderModule::spawn(&EntityID::External("MAIN".to_string()), module_b_id.clone(), execution_log.clone(), wait_duration_ms.clone()).await.unwrap();

    MMF::init_module(&module_a_id).await.unwrap();
    MMF::init_module(&module_b_id).await.unwrap();
    MMF::init_module(&module_c_id).await.unwrap();
    MMF::wait_module_list_init(&vec![module_a_id.clone(), module_b_id.clone(), module_c_id.clone()], Some(1000)).await.unwrap();

    MMF::send_message(&EntityID::External("MAIN".to_string()), &module_a_id, &MESSAGE_CHAIN.to_string(), Bytes::new()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    MMF::send_message(&EntityID::External("MAIN".to_string()), &module_a_id, &MESSAGE_NORMAL_1.to_string(), Bytes::new()).await.unwrap();
    MMF::send_message(&EntityID::External("MAIN".to_string()), &module_a_id, &MESSAGE_NORMAL_2.to_string(), Bytes::new()).await.unwrap();
    MMF::send_priority_message(&EntityID::External("MAIN".to_string()), &module_a_id, &MESSAGE_PRIORITY_1.to_string(), Bytes::new()).await.unwrap();
    MMF::send_priority_message(&EntityID::External("MAIN".to_string()), &module_a_id, &MESSAGE_PRIORITY_2.to_string(), Bytes::new()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(1000)).await;

    let duration = *wait_duration_ms.lock().unwrap();

    let duration = duration.unwrap();

    assert!(duration >= 450);
    assert!(duration <= 700);

    let log_guard = execution_log.lock().unwrap();

    println!("{:?}", *log_guard);

    assert_eq!(log_guard[0], MESSAGE_CHAIN);
    assert_eq!(log_guard[1], MESSAGE_PRIORITY_2);
    assert_eq!(log_guard[2], MESSAGE_PRIORITY_1);
    assert_eq!(log_guard[3], MESSAGE_NORMAL_1);
    assert_eq!(log_guard[4], MESSAGE_NORMAL_2);

    MMF::close_module(&module_a_id).await.unwrap();
    MMF::close_module(&module_b_id).await.unwrap();
    MMF::close_module(&module_c_id).await.unwrap();

    MMF::wait_children_close(&EntityID::External("MAIN".to_string()), Some(1000)).await.unwrap();

    MMF::close().unwrap();

}


// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_module_message_chain_and_priority() {

    MMF::init().unwrap();

    let execution_log =
        Arc::new(Mutex::new(Vec::<String>::new()));

    let wait_duration_ms =
        Arc::new(Mutex::new(None));

    let module_a_id: ModuleID;
    let module_b_id: ModuleID;
    let module_c_id: ModuleID;
    let message_id_temp: MessageID = Default::default();

    module_c_id = SimulationModule::spawn(&EntityID::External("MAIN".to_string()), 300, 300, SimulationMessageBehavior::Wait { message_delay_ms: (300) }, false, None).await.unwrap();
    module_b_id = SimulationModule::spawn(&EntityID::External("MAIN".to_string()), 200, 200, SimulationMessageBehavior::SendAndWaitReply { target_module_id: module_c_id.clone(), message_id: message_id_temp, timeout_ms: (Some(500)), message_delay_ms: (200) }, false, None).await.unwrap();
    module_a_id = MessageRecorderModule::spawn(&EntityID::External("MAIN".to_string()), module_b_id.clone(), execution_log.clone(), wait_duration_ms.clone()).await.unwrap();

    MMF::init_module(&module_a_id).await.unwrap();
    MMF::init_module(&module_b_id).await.unwrap();
    MMF::init_module(&module_c_id).await.unwrap();
    MMF::wait_module_list_init(&vec![module_a_id.clone(), module_b_id.clone(), module_c_id.clone()], Some(1000)).await.unwrap();

    MMF::send_message(&EntityID::External("MAIN".to_string()), &module_a_id, &MESSAGE_CHAIN.to_string(), Bytes::new()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    MMF::send_message(&EntityID::External("MAIN".to_string()), &module_a_id, &MESSAGE_NORMAL_1.to_string(), Bytes::new()).await.unwrap();
    MMF::send_message(&EntityID::External("MAIN".to_string()), &module_a_id, &MESSAGE_NORMAL_2.to_string(), Bytes::new()).await.unwrap();
    MMF::send_priority_message(&EntityID::External("MAIN".to_string()), &module_a_id, &MESSAGE_PRIORITY_1.to_string(), Bytes::new()).await.unwrap();
    MMF::send_priority_message_and_wait_reply(&EntityID::External("MAIN".to_string()), &module_a_id, &MESSAGE_PRIORITY_2.to_string(), Bytes::new(), Some(1000)).await.unwrap();

    tokio::time::sleep(Duration::from_millis(1000)).await;

    let duration = *wait_duration_ms.lock().unwrap();

    let duration = duration.unwrap();

    assert!(duration >= 450);
    assert!(duration <= 700);

    let log_guard = execution_log.lock().unwrap();

    println!("{:?}", *log_guard);

    assert_eq!(log_guard[0], MESSAGE_CHAIN);
    assert_eq!(log_guard[1], MESSAGE_PRIORITY_2);
    assert_eq!(log_guard[2], MESSAGE_PRIORITY_1);
    assert_eq!(log_guard[3], MESSAGE_NORMAL_1);
    assert_eq!(log_guard[4], MESSAGE_NORMAL_2);

    MMF::close_module(&module_a_id).await.unwrap();
    MMF::close_module(&module_b_id).await.unwrap();
    MMF::close_module(&module_c_id).await.unwrap();

    MMF::wait_children_close(&EntityID::External("MAIN".to_string()), Some(1000)).await.unwrap();

    MMF::close().unwrap();

}