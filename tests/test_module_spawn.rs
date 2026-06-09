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
        println!("on_init");
    }

    async fn on_message(
        &mut self,
        _message: FrameworkMessage,
    ) {
    }

    async fn on_close(&mut self) {
        println!("on_close");
    }
}

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_module_lifecycle() {

    MMF::init().unwrap();

    let module_id =
        MMF::spawn_module(
            TestModule,
            EntityID::External("TEST".to_string()),
        )
        .unwrap();

    assert_eq!(
        MMF::get_module_state(&module_id).unwrap(),
        ModuleState::Created,
    );

    MMF::init_module(&module_id)
        .await
        .unwrap();

    assert_eq!(
        MMF::get_module_state(&module_id).unwrap(),
        ModuleState::Initialized,
    );

    MMF::close_module(&module_id)
        .await
        .unwrap();

    assert!(
        matches!(
            MMF::get_module_state(&module_id),
            Err(FrameworkError::Module_NotFound)
        )
    );

    MMF::close().unwrap();
}