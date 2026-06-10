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
async fn test_module_hierarchy() {

    MMF::init().unwrap();

    let module_a_id =
        MMF::spawn_module(
            TestModule,
            EntityID::External("MAIN".to_string()),
        )
        .unwrap();

    let module_b_id =
        MMF::spawn_module(
            TestModule,
            EntityID::Module(module_a_id.clone()),
        )
        .unwrap();

    let module_c_id =
        MMF::spawn_module(
            TestModule,
            EntityID::Module(module_a_id.clone()),
        )
        .unwrap();

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

    MMF::close().unwrap();
}