use mesh_module_framework::message::FrameworkMessage;
use mesh_module_framework::mmf::MMF;
use mesh_module_framework::module::Module;

struct ModuleA;

struct ModuleB;

impl Module for ModuleA {

    async fn on_init(&mut self) {
    }

    async fn on_message(
        &mut self,
        message: FrameworkMessage,
    ) {
    }

    async fn on_close(&mut self) {
    }
}

impl Module for ModuleB {

    async fn on_init(&mut self) {
    }

    async fn on_message(
        &mut self,
        message: FrameworkMessage,
    ) {
    }

    async fn on_close(&mut self) {
    }
}


#[test]
fn test_spawn_modules() {

    assert!(MMF::init().is_ok());

    let module_a = ModuleA;
    let module_b = ModuleB;

    let module_a_id =
        MMF::spawn_module(
            None,
            module_a,
        );

    assert!(module_a_id.is_ok());

    let module_a_id = module_a_id.unwrap();

    let module_b_id =
        MMF::spawn_module(
            Some(module_a_id),
            module_b,
        );

    assert!(module_b_id.is_ok());

    assert!(MMF::close().is_ok());
}