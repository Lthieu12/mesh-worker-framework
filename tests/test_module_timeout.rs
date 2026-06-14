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

mod common;

use serial_test::serial;

use mesh_module_framework::*;
use common::simulation_module::{SimulationModule, SimulationMessageBehavior};

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_module_timeout_init() {
    MMF::init().unwrap();

    // Very long init
    let module_id = SimulationModule::spawn(&EntityID::External("MAIN".to_string()), 5_000, 0, SimulationMessageBehavior::DoNothing, false, None).await.unwrap();

    MMFMod::init_module(&module_id).await.unwrap();

    assert!(
        matches!(
            MMFMod::wait_module_init(&module_id, Some(100)).await,Err(FrameworkError::Operation_Timeout))
    );

    MMFMod::abort_module(&module_id).unwrap();
    MMF::close().unwrap();
}


// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_module_timeout_close() {
    MMF::init().unwrap();

    // Very long close
    let module_id = SimulationModule::spawn(&EntityID::External("MAIN".to_string()), 0, 5_000, SimulationMessageBehavior::DoNothing, false, None).await.unwrap();

    MMFMod::init_module(&module_id).await.unwrap();

    MMFMod::wait_module_init(&module_id, Some(1000)).await.unwrap();

    MMFMod::close_module(&module_id).await.unwrap();

    assert!(
        matches!(
            MMFMod::wait_module_close(&module_id, Some(100)).await, Err(FrameworkError::Operation_Timeout)
        )
    );

    MMFMod::abort_module(&module_id).unwrap();
    MMF::close().unwrap();
}


