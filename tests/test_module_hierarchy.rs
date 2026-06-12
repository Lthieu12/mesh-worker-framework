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
use common::simulation_module::SimulationModule;

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_module_hierarchy() {

    MMF::init().unwrap();

    let module_a_id = SimulationModule::spawn(EntityID::External("MAIN".to_string()), 100, 100, true, Some(500)).await.unwrap();
    println!("module a state = {:?}", MMF::get_module_state(&module_a_id));

    MMF::init_module(&module_a_id).await.unwrap();
    MMF::wait_module_init(&module_a_id, Some(500)).await.unwrap();

    let module_b_id = SimulationModule::spawn(EntityID::Module(module_a_id.clone()), 200, 200, false, None).await.unwrap();
    println!("module b state = {:?}", MMF::get_module_state(&module_b_id));
    let module_c_id = SimulationModule::spawn(EntityID::Module(module_a_id.clone()), 300, 300, false, None).await.unwrap();
    println!("module c state = {:?}", MMF::get_module_state(&module_c_id));
    
    MMF::init_module(&module_b_id).await.unwrap();
    MMF::init_module(&module_c_id).await.unwrap();

    MMF::wait_module_list_init(&vec![module_b_id.clone(), module_c_id.clone()], Some(1000)).await.unwrap();


    // Verify father relationships.

    assert_eq!(
        MMF::get_father_id(&module_a_id).unwrap(),
        EntityID::External("MAIN".to_string())
    );

    assert_eq!(
        MMF::get_father_id(&module_b_id).unwrap(),
        EntityID::Module(module_a_id.clone())
    );

    assert_eq!(
        MMF::get_father_id(&module_c_id).unwrap(),
        EntityID::Module(module_a_id.clone())
    );

    // Verify children relationships.

    let children_ids =
        MMF::get_children_ids(
            &EntityID::Module(module_a_id.clone())
        ).unwrap();

    assert_eq!(children_ids.len(), 2);
    assert!(children_ids.contains(&module_b_id));
    assert!(children_ids.contains(&module_c_id));

    let children_ids =
        MMF::get_children_ids(
            &EntityID::Module(module_b_id.clone())
        ).unwrap();

    assert!(children_ids.is_empty());

    let children_ids =
        MMF::get_children_ids(
            &EntityID::Module(module_c_id.clone())
        ).unwrap();

    assert!(children_ids.is_empty());

    let children_ids =
        MMF::get_children_ids(
            &EntityID::External("MAIN".to_string())
        ).unwrap();

    assert_eq!(children_ids.len(), 1);
    assert!(children_ids.contains(&module_a_id));

    println!("MAIN children(s) = {:?}", MMF::get_children_ids(&EntityID::External("MAIN".to_string())));
    println!("module_a children(s) = {:?}", MMF::get_children_ids(&EntityID::Module(module_a_id.clone())));

    MMF::close_module(&module_a_id).await.unwrap();
    MMF::close_module(&module_b_id).await.unwrap();
    MMF::close_module(&module_c_id).await.unwrap();

    MMF::wait_children_close(&EntityID::External("MAIN".to_string()), Some(1000)).await.unwrap();

    MMF::close().unwrap();
}