//! Validate MMF timeout and recovery mechanisms.
//!
//! Scenario:
//! - Simulate a module that takes too long to initialize.
//! - Verify wait_module_init() timeout detection.
//! - Simulate a module that takes too long to shutdown.
//! - Verify wait_module_close() timeout detection.
//! - Recover using kill_module().
//! - Shutdown MMF cleanly.
//!
//! This test validates timeout handling and the ability of MMF
//! to recover from unresponsive modules.

use bytes::Bytes;
use serial_test::serial;

use dev_support::simulation_module::{SimulationBehavior, SimulationModule, SimulationModuleConfig};
use mesh_module_framework::*;

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_module_timeout_init() {
    MMF::init().unwrap();
    MMF::register_module_type::<SimulationModule>().unwrap();

    let module_id = MMFMod::spawn_module(SimulationModule::module_type_id(), &EntityID::new_external("MAIN")).unwrap();

    // Very long init
    let configuration = SimulationModuleConfig { init_delay_ms: 5_000, close_delay_ms: 0, simulation_behavior: SimulationBehavior::DoNothing, wait_children_close: false, wait_children_close_timeout_ms: None };
    let lifecycle_data = Bytes::from(bincode::serialize(&configuration).unwrap());

    MMFMod::init_module(&module_id, lifecycle_data).await.unwrap();

    assert!(matches!(MMFMod::wait_module_init(&module_id, Some(100)).await, Err(MMFError::Operation_Timeout)));

    MMFMod::abort_module(&module_id).unwrap();
    MMF::close().unwrap();
}

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_module_timeout_close() {
    MMF::init().unwrap();
    MMF::register_module_type::<SimulationModule>().unwrap();

    let module_id = MMFMod::spawn_module(SimulationModule::module_type_id(), &EntityID::new_external("MAIN")).unwrap();

    // Very long close
    let configuration = SimulationModuleConfig { init_delay_ms: 0, close_delay_ms: 5_000, simulation_behavior: SimulationBehavior::DoNothing, wait_children_close: false, wait_children_close_timeout_ms: None };
    let lifecycle_data = Bytes::from(bincode::serialize(&configuration).unwrap());

    MMFMod::init_module(&module_id, lifecycle_data).await.unwrap();

    MMFMod::wait_module_init(&module_id, Some(1000)).await.unwrap();

    MMFMod::close_module(&module_id, Bytes::new()).await.unwrap();

    assert!(matches!(MMFMod::wait_module_close(&module_id, Some(100)).await, Err(MMFError::Operation_Timeout)));

    MMFMod::abort_module(&module_id).unwrap();
    MMF::close().unwrap();
}
