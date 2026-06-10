use async_trait::async_trait;
use serial_test::serial;

use mesh_module_framework::*;

struct TestModule;

#[async_trait]
impl Module for TestModule {

    async fn on_init(
        &mut self,
        _module_id: &ModuleID,
        _father_id: &EntityID,
    ) {
    }

    async fn on_message(
        &mut self,
        _message: FrameworkMessage,
    ) {
    }

    async fn on_close(
        &mut self,
    ) {
    }
}

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_wait_children_close_immediate() {

    MMF::init().unwrap();

    MMF::wait_children_close(
        &EntityID::External("MAIN".to_string()),
        Some(1000),
    )
    .await
    .unwrap();

    MMF::close().unwrap();
}

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_wait_children_close_after_close() {

    MMF::init().unwrap();

    let module_id =
        MMF::spawn_module(
            TestModule,
            EntityID::External("MAIN".to_string()),
        )
        .unwrap();

    MMF::init_module(&module_id)
        .await
        .unwrap();

    let close_module_id = module_id.clone();

    let wait_task = async {

        MMF::wait_children_close(
            &EntityID::External("MAIN".to_string()),
            Some(1000),
        )
        .await
    };

    let close_task = async move {

        tokio::time::sleep(
            std::time::Duration::from_millis(100),
        )
        .await;

        MMF::close_module(&close_module_id)
            .await
            .unwrap();
    };

    let (wait_result, _) =
        tokio::join!(wait_task, close_task);

    wait_result.unwrap();

    MMF::close().unwrap();
}

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_wait_children_close_timeout() {

    MMF::init().unwrap();

    let module_id =
        MMF::spawn_module(
            TestModule,
            EntityID::External("MAIN".to_string()),
        )
        .unwrap();

    MMF::init_module(&module_id)
        .await
        .unwrap();

    let result =
        MMF::wait_children_close(
            &EntityID::External("MAIN".to_string()),
            Some(100),
        )
        .await;

    assert!(matches!(
        result,
        Err(FrameworkError::Operation_Timeout)
    ));

    MMF::close().unwrap();
}

