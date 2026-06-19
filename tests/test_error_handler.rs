/// Verify that MMFErr captures and reports message
/// processing failures.
///
/// A SimulationModule configured with Message_Fail is
/// initialized and receives several messages.
///
/// Each failure should:
/// - Generate an ErrorEvent.
/// - Wake MMFErr waiters.
/// - Be retrievable through MMFErr::get_error_list().
///
/// The test also verifies that ErrorInstanceID values
/// are strictly increasing.
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
async fn test_mmf_err_message_failure() {

    MMF::init().unwrap();

    let module_id =
        SimulationModule::spawn(
            &EntityID::External("Test".to_string()),
            0,
            0,
            SimulationBehavior::Message_Fail,
            false,
            None,
        )
        .await
        .unwrap();

    MMFMod::init_module(&module_id).await.unwrap();
    MMFMod::wait_module_init(&module_id, Some((1000))).await.unwrap();

    // Generate multiple message failures.
    for _ in 0..3 {


        MMFMod::send_message(
            &EntityID::External("Test".to_string()),
            &module_id,
            &"TEST".to_string(),
            Bytes::new(),
        )
        .await
        .unwrap();

        MMFErr::wait_error().await.unwrap();
    }

    let error_list =
        MMFErr::get_error_list(None).unwrap();

    assert_eq!(error_list.missing_error_count, 0);

    assert_eq!(
        error_list.error_event_list.len(),
        3
    );

    assert!(
        error_list.first_error_instance_id.is_some()
    );

    assert!(
        error_list.last_error_instance_id.is_some()
    );

    for error_event in &error_list.error_event_list {

        assert_eq!(
            error_event.error_message,
            "Simulation message failure"
        );

        assert!(
            matches!(
                error_event.sender_id,
                EntityID::Module(_)
            )
        );

        assert!(
            !error_event.timestamp_rfc3339.is_empty()
        );
    }

    // Verify ordering of ErrorInstanceID.
    for window in error_list.error_event_list.windows(2) {

        assert!(
            window[0].error_instance_id
                < window[1].error_instance_id
        );
    }

    MMFMod::abort_module(&module_id).unwrap();

    MMF::close().unwrap();
}
