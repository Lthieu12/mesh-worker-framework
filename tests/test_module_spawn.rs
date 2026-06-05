use mesh_module_framework::mmf::MMF;
use mesh_module_framework::module::Module;
use mesh_module_framework::message::FrameworkMessage;

struct TestModule;

impl Module for TestModule {

    async fn on_init(&mut self) {}

    async fn on_message(
        &mut self,
        _message: FrameworkMessage,
    ) {}

    async fn on_close(&mut self) {}
}

#[tokio::test]
async fn test_spawn_module() {

    assert!(MMF::init().is_ok());

    let module_id =
        MMF::spawn_module(
            TestModule,
            None,
        );

    assert!(module_id.is_ok());

    let module_id = module_id.unwrap();

    assert!(!module_id.is_empty());

    assert!(MMF::close().is_ok());
}

#[tokio::test]
async fn test_spawn_module_with_father() {

    assert!(MMF::init().is_ok());

    let father_id =
        MMF::spawn_module(
            TestModule,
            None,
        ).unwrap();

    let child_id =
        MMF::spawn_module(
            TestModule,
            Some(father_id),
        );

    assert!(child_id.is_ok());

    assert!(MMF::close().is_ok());
}

