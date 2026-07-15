//! Validate module registration and lifecycle failure handling.
//!
//! These tests verify that:
//! - Module types must be registered before spawning.
//! - Initialization failures are correctly propagated.
//! - Shutdown failures are correctly propagated.

use bytes::Bytes;
use serial_test::serial;

use dev_support::simulation_module::{SimulationBehavior, SimulationModule, SimulationModuleConfig};
use mesh_module_framework::*;

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_module_init_failed() {
    MMF::init().unwrap();

    let result = MMFMod::spawn_module("SimulationModule", &EntityID::External("MAIN".into()));

    assert!(matches!(result, Err(FrameworkError::ModuleType_NotRegistered)));

    MMF::register_module_type::<SimulationModule>().unwrap();

    let module_id = MMFMod::spawn_module("SimulationModule", &EntityID::External("MAIN".into())).unwrap();

    let configuration = SimulationModuleConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::Init_Fail, wait_children_close: false, wait_children_close_timeout_ms: None };
    let lifecycle_data = Bytes::from(bincode::serialize(&configuration).unwrap());

    MMFMod::init_module(&module_id, lifecycle_data).await.unwrap();

    let module_id = MMFMod::wait_module_list_init(&vec![module_id.clone()], Some(1000)).await;
    println!("Result: {:?}", module_id);
    assert!(matches!(module_id, Err(FrameworkError::Module_InitializationFailed)));

    MMFMod::abort_children(&EntityID::External("MAIN".into())).unwrap();

    MMF::close().unwrap();
}

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_module_close_failed() {
    MMF::init().unwrap();

    MMF::register_module_type::<SimulationModule>().unwrap();

    let module_id = MMFMod::spawn_module("SimulationModule", &EntityID::External("MAIN".into())).unwrap();

    let configuration = SimulationModuleConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::Close_Fail, wait_children_close: false, wait_children_close_timeout_ms: None };
    let lifecycle_data = Bytes::from(bincode::serialize(&configuration).unwrap());

    MMFMod::init_module(&module_id, lifecycle_data).await.unwrap();

    MMFMod::wait_module_list_init(&vec![module_id.clone()], Some(1000)).await.unwrap();

    MMFMod::close_module(&module_id, Bytes::new()).await.unwrap();

    let result = MMFMod::wait_module_list_close(&vec![module_id.clone()], Some(1000)).await;

    println!("Result: {:?}", result);
    assert!(matches!(result, Err(FrameworkError::Module_CloseFailed)));

    MMFMod::abort_children(&EntityID::External("MAIN".into())).unwrap();

    MMF::close().unwrap();
}
