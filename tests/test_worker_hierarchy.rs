//! Validate the complete MWF worker lifecycle on a small hierarchy.
//!
//! Scenario:
//! - Create a parent worker and two child workers.
//! - Verify parent/child relationships.
//! - Initialize all workers.
//! - Shutdown all workers.
//! - Wait for complete worker removal.
//! - Shutdown MWF.
//!
//! This test validates the normal lifecycle path of the framework,
//! including hierarchy management, initialization, shutdown and
//! synchronization helpers.

use bytes::Bytes;
use serial_test::serial;

use dev_support::dev_helper::*;
use dev_support::simulation_worker::{SimulationBehavior, SimulationWorker, SimulationWorkerConfig};
use mesh_worker_framework::*;

// MWF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_worker_hierarchy() {
    MWF::init().unwrap();
    MWF::register_worker_type::<SimulationWorker>().unwrap();

    let worker_a_id = MWFWrk::spawn_worker(SimulationWorker::worker_type_id(), &EntityID::new_external("MAIN")).unwrap();

    let configuration = SimulationWorkerConfig { init_delay_ms: 100, close_delay_ms: 100, simulation_behavior: SimulationBehavior::DoNothing, wait_children_close: true, wait_children_close_timeout_ms: Some(500) };
    let lifecycle_data = Bytes::from(bincode::serialize(&configuration).unwrap());

    MWFWrk::init_worker(&worker_a_id, lifecycle_data).await.unwrap();

    MWFWrk::wait_worker_init(&worker_a_id, Some(500)).await.unwrap();

    let worker_b_id = MWFWrk::spawn_worker(SimulationWorker::worker_type_id(), &EntityID::Worker(worker_a_id.clone())).unwrap();

    let configuration = SimulationWorkerConfig { init_delay_ms: 200, close_delay_ms: 200, simulation_behavior: SimulationBehavior::DoNothing, wait_children_close: false, wait_children_close_timeout_ms: None };
    let lifecycle_data = Bytes::from(bincode::serialize(&configuration).unwrap());

    MWFWrk::init_worker(&worker_b_id, lifecycle_data).await.unwrap();

    let worker_c_id = MWFWrk::spawn_worker(SimulationWorker::worker_type_id(), &EntityID::Worker(worker_a_id.clone())).unwrap();

    let configuration = SimulationWorkerConfig { init_delay_ms: 300, close_delay_ms: 300, simulation_behavior: SimulationBehavior::DoNothing, wait_children_close: false, wait_children_close_timeout_ms: None };
    let lifecycle_data = Bytes::from(bincode::serialize(&configuration).unwrap());

    MWFWrk::init_worker(&worker_c_id, lifecycle_data).await.unwrap();

    MWFWrk::wait_worker_list_init(&vec![worker_b_id.clone(), worker_c_id.clone()], Some(1000)).await.unwrap();

    print_worker_list();

    // Verify father relationships.

    assert_eq!(MWFWrk::get_parent_id(&worker_a_id).unwrap(), EntityID::External("MAIN".into()));

    assert_eq!(MWFWrk::get_parent_id(&worker_b_id).unwrap(), EntityID::Worker(worker_a_id.clone()));

    assert_eq!(MWFWrk::get_parent_id(&worker_c_id).unwrap(), EntityID::Worker(worker_a_id.clone()));

    // Verify children relationships.

    let children_ids = MWFWrk::get_children_ids(&EntityID::Worker(worker_a_id.clone())).unwrap();

    assert_eq!(children_ids.len(), 2);
    assert!(children_ids.contains(&worker_b_id));
    assert!(children_ids.contains(&worker_c_id));

    let children_ids = MWFWrk::get_children_ids(&EntityID::Worker(worker_b_id.clone())).unwrap();

    assert!(children_ids.is_empty());

    let children_ids = MWFWrk::get_children_ids(&EntityID::Worker(worker_c_id.clone())).unwrap();

    assert!(children_ids.is_empty());

    let children_ids = MWFWrk::get_children_ids(&EntityID::new_external("MAIN")).unwrap();

    assert_eq!(children_ids.len(), 1);
    assert!(children_ids.contains(&worker_a_id));

    println!("MAIN children(s) = {:?}", MWFWrk::get_children_ids(&EntityID::new_external("MAIN")));
    println!("worker_a children(s) = {:?}", MWFWrk::get_children_ids(&EntityID::Worker(worker_a_id.clone())));

    MWFWrk::close_worker(&worker_a_id, Bytes::new()).await.unwrap();
    MWFWrk::close_worker(&worker_b_id, Bytes::new()).await.unwrap();
    MWFWrk::close_worker(&worker_c_id, Bytes::new()).await.unwrap();

    MWFWrk::wait_children_close(&EntityID::new_external("MAIN"), Some(1000)).await.unwrap();

    MWF::close().unwrap();
}
