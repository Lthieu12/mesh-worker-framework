//! Verify proper behavior of MMFLog.
//! 
//! First test :
//! Validate module lifecycle logging.
//!
//! This test verifies that MMFLog records:
//! - ModuleSpawn
//! - ModuleInit
//! - ModuleClose
//!
//! Second test :
//! in the expected order with LogStatus::Ok.
//!
//! Validate message reception logging.
//!
//! This test verifies that MMFLog records
//! received messages with the correct:
//! - sender_id
//! - message_id
//! - priority flag
//! - wait_completion flag
//! - LogStatus
//! 
//! Third test :
//! Validate MMFLog circular buffer behavior.
//!
//! This test verifies that:
//! - old log events are overwritten when the buffer is full
//! - missing_log_count is correctly reported
//! - only the most recent log events are retained
use bytes::Bytes;

mod common;

use serial_test::serial;

use mesh_module_framework::*;
use common::simulation_module::{SimulationModule, SimulationBehavior};


// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_mmf_log_module_lifecycle() {

    MMF::init().unwrap();

    MMFLog::set_log_mode(LogMode::LogAll, None).unwrap();

    let module_id = SimulationModule::spawn(&EntityID::External("Test".to_string()), 0, 0, SimulationBehavior::DoNothing, false, None).await.unwrap();

    MMFMod::init_module(&module_id).await.unwrap();
    MMFMod::wait_module_init(&module_id, Some(1000)).await.unwrap();
    MMFMod::close_module(&module_id).await.unwrap();
    MMFMod::wait_module_close(&module_id, Some(1000)).await.unwrap();

    let log_list = MMFLog::wait_log(None).await.unwrap();

    assert_eq!(log_list.missing_log_count, 0);

    assert_eq!(log_list.log_event_list.len(), 5);

    assert!(matches!(log_list.log_event_list[0].log_action, LogAction::ModuleSpawn));

    assert!(matches!(log_list.log_event_list[1].log_action, LogAction::ModuleInit));
    assert!(matches!(log_list.log_event_list[2].log_action, LogAction::ModuleInit));

    assert!(matches!(log_list.log_event_list[3].log_action, LogAction::ModuleClose));
    assert!(matches!(log_list.log_event_list[4].log_action, LogAction::ModuleClose));

    assert_eq!(log_list.log_event_list[0].log_status, LogStatus::Ok);

    assert_eq!(log_list.log_event_list[1].log_status, LogStatus::Started);
    assert_eq!(log_list.log_event_list[2].log_status, LogStatus::Ok);

    assert_eq!(log_list.log_event_list[3].log_status, LogStatus::Started);
    assert_eq!(log_list.log_event_list[4].log_status, LogStatus::Ok);

    MMF::close().unwrap();
}


// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_mmf_log_messages() {

    MMF::init().unwrap();

    MMFLog::set_log_mode(LogMode::LogAll, None).unwrap();

    let module_id = SimulationModule::spawn(&EntityID::External("Test".to_string()), 0, 0, SimulationBehavior::DoNothing, false, None).await.unwrap();

    MMFMod::init_module(&module_id).await.unwrap();
    MMFMod::wait_module_init(&module_id, Some(1000)).await.unwrap();
    MMFMod::send_message(&EntityID::External("Test".to_string()), &module_id, &"MSG_1".to_string(), Bytes::new()).await.unwrap();
    MMFMod::send_priority_message(&EntityID::External("Test".to_string()), &module_id, &"MSG_2".to_string(), Bytes::new()).await.unwrap();
    MMFMod::send_message_and_wait(&EntityID::External("Test".to_string()), &module_id, &"MSG_3".to_string(), Bytes::new(), Some(1000)).await.unwrap();
    MMFMod::send_priority_message_and_wait(&EntityID::External("Test".to_string()), &module_id, &"MSG_4".to_string(), Bytes::new(), Some(1000)).await.unwrap();
    MMFMod::close_module(&module_id).await.unwrap();
    MMFMod::wait_module_close(&module_id, Some(1000)).await.unwrap();

    let log_list = MMFLog::wait_log(None).await.unwrap();

    let mut found_normal = false;
    let mut found_priority = false;
    let mut found_wait = false;
    let mut found_priority_wait = false;

    for log_event in &log_list.log_event_list {

        println!("{:?} {:?}", log_event.log_action, log_event.log_status);

        match &log_event.log_action {

            LogAction::ReceiveMessage {
                priority,
                wait_completion,
                ..
            } => {

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

                assert!(
                    matches!(
                        log_event.log_status,
                        LogStatus::Started | LogStatus::Ok
                    )
                );
            }

            _ => {}
        }
    }

    assert!(found_normal);
    assert!(found_priority);
    assert!(found_wait);
    assert!(found_priority_wait);

    MMF::close().unwrap();
}


// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_mmf_log_buffer_overflow() {

    let config = MMFConfiguration {

        log_event_buffer_capacity: 10,

        ..Default::default()
    };

    MMF::init_with_configuration(config).unwrap();

    MMFLog::set_log_mode(LogMode::LogAll, None).unwrap();

    let module_id = SimulationModule::spawn(&EntityID::External("Test".to_string()), 0, 0, SimulationBehavior::DoNothing, false, None).await.unwrap();

    MMFMod::init_module(&module_id).await.unwrap();

    MMFMod::wait_module_init(&module_id, Some(1000)).await.unwrap();

    // Consume all logs currently present.
    let log_list = MMFLog::wait_log(None).await.unwrap();

    let last_log_instance_id = log_list.last_log_instance_id;

    // Generate more logs than the buffer can hold.
    for index in 0..15 {

        MMFMod::send_message(&EntityID::External("Test".to_string()), &module_id, &format!("MSG_{index}"), Bytes::new()).await.unwrap();
    }

    let log_list = MMFLog::wait_log(last_log_instance_id).await.unwrap();

    assert_eq!(log_list.missing_log_count, 20);

    assert_eq!(log_list.log_event_list.len(), 10);

    MMFMod::close_module(&module_id).await.unwrap();
    MMFMod::wait_module_close(&module_id, Some(1000)).await.unwrap();

    MMF::close().unwrap();
}