//! Tests for MMF blocking task helpers.

use serial_test::serial;

use mesh_module_framework::*;


#[tokio::test]
#[serial]
async fn test_spawn_blocking_ok() {

    MMF::init().unwrap();

    let result = MMFMod::spawn_blocking(|| {

        42

    }).await.unwrap();

    assert_eq!(result, 42);

    MMF::close().unwrap();
}


#[tokio::test]
#[serial]
async fn test_spawn_blocking_panic() {

    MMF::init().unwrap();

    let result = MMFMod::spawn_blocking(|| {

        panic!("Blocking task panic");

    }).await;

    assert!(matches!(
        result,
        Err(FrameworkError::BlockingTask_Failed(_))
    ));

    MMF::close().unwrap();
}