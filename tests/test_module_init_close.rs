//! Validate module registration and lifecycle failure handling.
//!
//! These tests verify that:
//! - Module types must be registered before spawning.
//! - Initialization failures are correctly propagated.
//! - Shutdown failures are correctly propagated.
mod common;

use serial_test::serial;

use mesh_module_framework::*;
use common::simulation_module::{SimulationModule, SimulationBehavior};

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_module_init_failed() {

    MMF::init().unwrap();

    let result = SimulationModule::spawn(&EntityID::External("MAIN".to_string()), 0, 0, SimulationBehavior::Init_Fail, false, None).await;

    assert!(matches!(result, Err(FrameworkError::ModuleType_NotRegistered)));

    MMF::register_module_type::<SimulationModule>().unwrap();

    let module_id = SimulationModule::spawn(&EntityID::External("MAIN".to_string()), 0, 0, SimulationBehavior::Init_Fail, false, None).await.unwrap();

    MMFMod::init_module(&module_id).await.unwrap();
    let module_id = MMFMod::wait_module_list_init(&vec![module_id.clone()], Some(1000)).await;
    println!("Result: {:?}", module_id);
    assert!(matches!(module_id, Err(FrameworkError::Module_InitializationFailed)));

    MMFMod::abort_children(&EntityID::External("MAIN".to_string())).unwrap();

    MMF::close().unwrap();
}

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_module_close_failed() {

    MMF::init().unwrap();

    MMF::register_module_type::<SimulationModule>().unwrap();

    let module_id = SimulationModule::spawn(&EntityID::External("MAIN".to_string()), 0, 0, SimulationBehavior::Close_Fail, false, None).await.unwrap();

    MMFMod::init_module(&module_id).await.unwrap();

    MMFMod::wait_module_list_init(&vec![module_id.clone()], Some(1000)).await.unwrap();

    MMFMod::close_module(&module_id).await.unwrap();

    let result = MMFMod::wait_module_list_close(&vec![module_id.clone()], Some(1000)).await;

    println!("Result: {:?}", result);
    assert!(matches!(result, Err(FrameworkError::Module_CloseFailed)));

    MMFMod::abort_children(&EntityID::External("MAIN".to_string())).unwrap();

    MMF::close().unwrap();
}