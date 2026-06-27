//! Verify that MMF recovers from a module panic.
//!
//! The panicked module must be removed from the Registry,
//! the panic must be reported, and the runtime must remain
//! operational.

mod common;

use serial_test::serial;

use mesh_module_framework::*;
use common::simulation_module::{SimulationBehavior, SimulationModule};

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_module_message_panic() {

    MMF::init().unwrap();

    MMFLog::set_log_mode(LogMode::LogAll, None).unwrap();

    let module_id = SimulationModule::spawn(&EntityID::External("MAIN".to_string()), 0, 0, SimulationBehavior::Message_Panic, false, None).await.unwrap();

    MMFMod::init_module(&module_id).await.unwrap();
    MMFMod::wait_module_init(&module_id, Some(1000)).await.unwrap();

    // Trigger the panic.
    MMFMod::send_message(&EntityID::External("MAIN".to_string()), &module_id, &"TEST".to_string(), bytes::Bytes::new()).await.unwrap();

    // Wait until MMF has removed the module.
    _ = MMFMod::wait_module_close(&module_id, Some(1000)).await;

    // The module must no longer exist.
    assert!(MMFMod::get_module_status(&module_id).is_err());

    // Wait for the panic to be recorded.
    let error_list = MMFErr::wait_error(None).await.unwrap();

    assert_eq!(error_list.error_event_list.len(), 1);

    assert!(error_list.error_event_list[0].error_message.contains("Simulation message panic"));

    // One panic event must be present in the log buffer.
    let log_list = MMFLog::wait_log(None).await.unwrap();

    assert!(log_list.log_event_list.iter().any(|log| matches!(log.log_action, LogAction::ModulePanic)));

    MMF::close().unwrap();
}