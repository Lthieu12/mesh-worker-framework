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
async fn test_wait_children_close_stress_live() {

    MMF::init().unwrap();

    for iteration in 0..100 {

        let root =
            EntityID::External(
                format!("ROOT_{iteration}")
            );

        let mut module_ids = Vec::new();

        for _ in 0..20 {

            let module_id =
                MMF::spawn_module(
                    TestModule,
                    root.clone(),
                )
                .unwrap();

            MMF::init_module(&module_id)
                .await
                .unwrap();

            module_ids.push(module_id);
        }

        let wait_task = async {

            MMF::wait_children_close(
                &root,
                Some(5000),
            )
            .await
        };

        let close_task = async move {

            for module_id in module_ids {

                MMF::close_module(&module_id)
                    .await
                    .unwrap();
            }
        };

        let (wait_result, _) =
            tokio::join!(wait_task, close_task);

        wait_result.unwrap();
    }

    MMF::close().unwrap();
}

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_wait_children_close_multiple_waiters() {

    MMF::init().unwrap();

    for iteration in 0..100 {

        let root =
            EntityID::External(
                format!("ROOT_{iteration}")
            );

        let mut module_ids = Vec::new();

        for _ in 0..20 {

            let module_id =
                MMF::spawn_module(
                    TestModule,
                    root.clone(),
                )
                .unwrap();

            MMF::init_module(&module_id)
                .await
                .unwrap();

            module_ids.push(module_id);
        }

        let waiter_1 = async {
            MMF::wait_children_close(
                &root,
                Some(5000),
            )
            .await
        };

        let waiter_2 = async {
            MMF::wait_children_close(
                &root,
                Some(5000),
            )
            .await
        };

        let waiter_3 = async {
            MMF::wait_children_close(
                &root,
                Some(5000),
            )
            .await
        };

        let close_task = async move {

            for module_id in module_ids {

                MMF::close_module(&module_id)
                    .await
                    .unwrap();
            }
        };

        let (
            result_1,
            result_2,
            result_3,
            _
        ) = tokio::join!(
            waiter_1,
            waiter_2,
            waiter_3,
            close_task
        );

        result_1.unwrap();
        result_2.unwrap();
        result_3.unwrap();
    }

    MMF::close().unwrap();
}