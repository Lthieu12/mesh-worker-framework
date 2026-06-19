/// Verify that MMFErr records callback failures.
///
/// This test generates failures from module lifecycle
/// callbacks (on_init and on_close) as well as business
/// message processing (on_message) and verifies that
/// the resulting error events can be retrieved through
/// MMFErr.
mod common;

use serial_test::serial;

use mesh_module_framework::*;
use common::simulation_module::{
    SimulationBehavior,
    SimulationModule,
};

use bytes::Bytes;


// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_mmf_err_module_failures() {

    MMF::init().unwrap();

    // Simulation module with error on message
    let module_a_id = SimulationModule::spawn(&EntityID::External("Test".to_string()), 0, 0, SimulationBehavior::Message_Fail, false, None).await.unwrap();
    MMFMod::init_module(&module_a_id).await.unwrap();
    MMFMod::wait_module_init(&module_a_id, Some(1000)).await.unwrap();
    // Generate multiple message failures.
    for _ in 0..3 {

        MMFMod::send_message(&EntityID::External("Test".to_string()), &module_a_id, &"TEST".to_string(), Bytes::new()).await.unwrap();
    }

    // Simulation module with error on init
    let module_b_id = SimulationModule::spawn(&EntityID::External("Test".to_string()), 0, 0, SimulationBehavior::Init_Fail, false, None).await.unwrap();
    MMFMod::init_module(&module_b_id).await.unwrap();
    let result = MMFMod::wait_module_init(&module_b_id, Some(1000)).await;
    assert!(
        matches!(
            result,
            Err(FrameworkError::Module_InitializationFailed)
        )
    );

    // Simulation module with error on close
    let module_c_id = SimulationModule::spawn(&EntityID::External("Test".to_string()), 0, 0, SimulationBehavior::Close_Fail, false, None).await.unwrap();
    MMFMod::init_module(&module_c_id).await.unwrap();
    MMFMod::wait_module_init(&module_c_id, Some(1000)).await.unwrap();
    MMFMod::close_module(&module_c_id).await.unwrap();
    let result = MMFMod::wait_module_close(&module_c_id, Some(1000)).await;
    assert!(
        matches!(
            result,
            Err(FrameworkError::Module_CloseFailed)
        )
    );

    let error_list = MMFErr::wait_error(None).await.unwrap();

    assert_eq!(error_list.missing_error_count, 0);

    assert_eq!(error_list.error_event_list.len(), 5);

    assert!(error_list.first_error_instance_id.is_some());

    assert!(error_list.last_error_instance_id.is_some());

    let error_messages: Vec<&str> = error_list.error_event_list.iter().map(|error_event| error_event.error_message.as_str()).collect();

    assert_eq!(error_messages.iter().filter(|message| **message == "Simulation message failure").count(), 3);

    assert_eq!(error_messages.iter().filter(|message| **message == "Simulation init failure").count(), 1);

    assert_eq!(error_messages.iter().filter(|message| **message == "Simulation close failure").count(), 1);
    
    for error_event in &error_list.error_event_list {

        assert!(
            matches!(
                error_event.sender_id,
                EntityID::Module(_)
            )
        );

        assert!(!error_event.timestamp_rfc3339.is_empty());
    }

    // Verify ordering of ErrorInstanceID.
    for window in error_list.error_event_list.windows(2) {

        assert!(window[0].error_instance_id < window[1].error_instance_id);
    }

    MMFMod::abort_module(&module_a_id).unwrap();
    MMFMod::abort_module(&module_b_id).unwrap();
    MMFMod::abort_module(&module_c_id).unwrap();

    MMF::close().unwrap();
}
