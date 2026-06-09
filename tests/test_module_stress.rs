use async_trait::async_trait;
use bytes::Bytes;
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
async fn test_module_stress() {

    MMF::init().unwrap();

    for iteration in 0..1000 {

        let module_id =
            MMF::spawn_module(
                TestModule,
                EntityID::External(
                    format!("TEST_{iteration}")
                ),
            )
            .unwrap();

        MMF::init_module(&module_id)
            .await
            .unwrap();

        MMF::send_message(
            &EntityID::External(
                "TEST".to_string()
            ),
            &module_id,
            &"Ping".to_string(),
            Bytes::new(),
        )
        .await
        .unwrap();

        MMF::close_module(&module_id)
            .await
            .unwrap();
    }

    MMF::close().unwrap();
}


// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_module_stress_multiple_modules() {

    MMF::init().unwrap();

    for batch in 0..100 {

        let mut module_ids = Vec::new();

        for i in 0..10 {

            let module_id =
                MMF::spawn_module(
                    TestModule,
                    EntityID::External(
                        format!("BATCH_{batch}")
                    ),
                )
                .unwrap();

            module_ids.push(module_id);
        }

        for module_id in &module_ids {

            MMF::init_module(module_id)
                .await
                .unwrap();
        }

        for module_id in &module_ids {

            MMF::send_message(
                &EntityID::External(
                    "TEST".to_string()
                ),
                module_id,
                &"Ping".to_string(),
                Bytes::new(),
            )
            .await
            .unwrap();
        }

        for module_id in &module_ids {

            MMF::close_module(module_id)
                .await
                .unwrap();
        }
    }

    MMF::close().unwrap();
}