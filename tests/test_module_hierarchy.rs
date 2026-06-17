//! Validate the complete MMF module lifecycle on a small hierarchy.
//!
//! Scenario:
//! - Create a parent module and two child modules.
//! - Verify parent/child relationships.
//! - Initialize all modules.
//! - Shutdown all modules.
//! - Wait for complete module removal.
//! - Shutdown MMF.
//!
//! This test validates the normal lifecycle path of the framework,
//! including hierarchy management, initialization, shutdown and
//! synchronization helpers.

mod common;

use serial_test::serial;

use mesh_module_framework::*;
use common::simulation_module::{SimulationModule, SimulationBehavior};

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_module_hierarchy() {

    MMF::init().unwrap();

    let module_a_id = SimulationModule::spawn(&EntityID::External("MAIN".to_string()), 100, 100, SimulationBehavior::DoNothing, true, Some(500)).await.unwrap();
    println!("module a state = {:?}", MMFMod::get_module_status(&module_a_id).unwrap().module_state);

    MMFMod::init_module(&module_a_id).await.unwrap();
    MMFMod::wait_module_init(&module_a_id, Some(500)).await.unwrap();

    let module_b_id = SimulationModule::spawn(&EntityID::Module(module_a_id.clone()), 200, 200, SimulationBehavior::DoNothing, false, None).await.unwrap();
    println!("module b state = {:?}", MMFMod::get_module_status(&module_b_id).unwrap().module_state);
    let module_c_id = SimulationModule::spawn(&EntityID::Module(module_a_id.clone()), 300, 300, SimulationBehavior::DoNothing, false, None).await.unwrap();
    println!("module c state = {:?}", MMFMod::get_module_status(&module_c_id).unwrap().module_state);
    
    MMFMod::init_module(&module_b_id).await.unwrap();
    MMFMod::init_module(&module_c_id).await.unwrap();

    MMFMod::wait_module_list_init(&vec![module_b_id.clone(), module_c_id.clone()], Some(1000)).await.unwrap();


    // Verify father relationships.

    assert_eq!(
        MMFMod::get_father_id(&module_a_id).unwrap(),
        EntityID::External("MAIN".to_string())
    );

    assert_eq!(
        MMFMod::get_father_id(&module_b_id).unwrap(),
        EntityID::Module(module_a_id.clone())
    );

    assert_eq!(
        MMFMod::get_father_id(&module_c_id).unwrap(),
        EntityID::Module(module_a_id.clone())
    );

    // Verify children relationships.

    let children_ids =
        MMFMod::get_children_ids(
            &EntityID::Module(module_a_id.clone())
        ).unwrap();

    assert_eq!(children_ids.len(), 2);
    assert!(children_ids.contains(&module_b_id));
    assert!(children_ids.contains(&module_c_id));

    let children_ids =
        MMFMod::get_children_ids(
            &EntityID::Module(module_b_id.clone())
        ).unwrap();

    assert!(children_ids.is_empty());

    let children_ids =
        MMFMod::get_children_ids(
            &EntityID::Module(module_c_id.clone())
        ).unwrap();

    assert!(children_ids.is_empty());

    let children_ids =
        MMFMod::get_children_ids(
            &EntityID::External("MAIN".to_string())
        ).unwrap();

    assert_eq!(children_ids.len(), 1);
    assert!(children_ids.contains(&module_a_id));

    println!("MAIN children(s) = {:?}", MMFMod::get_children_ids(&EntityID::External("MAIN".to_string())));
    println!("module_a children(s) = {:?}", MMFMod::get_children_ids(&EntityID::Module(module_a_id.clone())));

    MMFMod::close_module(&module_a_id).await.unwrap();
    MMFMod::close_module(&module_b_id).await.unwrap();
    MMFMod::close_module(&module_c_id).await.unwrap();

    MMFMod::wait_children_close(&EntityID::External("MAIN".to_string()), Some(1000)).await.unwrap();

    MMF::close().unwrap();
}