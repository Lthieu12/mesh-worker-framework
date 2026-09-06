//! Tests for MWF blocking task helpers.

use serial_test::serial;

use mwf::*;

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_spawn_blocking_ok() {
    MWF::init().unwrap();

    let result = MWFWrk::spawn_blocking(|| 42).await.unwrap();

    assert_eq!(result, 42);

    MWF::close().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_spawn_blocking_panic() {
    MWF::init().unwrap();

    let result = MWFWrk::spawn_blocking(|| {
        panic!("Blocking task panic");
    })
    .await;

    assert!(matches!(result, Err(MWFError::BlockingTask_Failed(_))));

    MWF::close().unwrap();
}
