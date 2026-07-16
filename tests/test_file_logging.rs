//! Verifies persistent file logging, including file creation,
//! log writing and automatic shutdown when the maximum file
//! size is reached.

use serial_test::serial;

use dev_support::simulation_module::{SimulationBehavior, SimulationModule, SimulationModuleConfig};
use mesh_module_framework::*;

// MMF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test]
#[serial]
async fn test_file_logging() {
    use bytes::Bytes;
    use std::fs;

    // Configure MMF.
    let mut configuration = MMFConfiguration::default();
    configuration.log_file_max_size_bytes = 4096;

    MMF::init_with_configuration(configuration).unwrap();

    let log_file_path = MMFLog::get_log_file_path().unwrap();

    MMF::register_module_type::<SimulationModule>().unwrap();

    MMFLog::set_log_mode(LogMode::LogAll, None).unwrap();
    MMFLog::set_log_file_enabled(true).unwrap();

    let simulation_configuration = SimulationModuleConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::DoNothing, wait_children_close: false, wait_children_close_timeout_ms: None };

    let lifecycle_data = Bytes::from(bincode::serialize(&simulation_configuration).unwrap());

    let module_id = MMFMod::spawn_module(SimulationModule::module_type_id(), &EntityID::new_external("Test")).unwrap();

    MMFMod::init_module(&module_id, lifecycle_data.clone()).await.unwrap();

    MMFMod::wait_module_init(&module_id, None).await.unwrap();

    // Generate enough log events to fill the log file.
    for index in 0..100 {
        MMFMod::send_message(&EntityID::new_external("Test"), &module_id, &format!("Message{index}"), Bytes::new()).await.unwrap();
    }

    MMFMod::close_module(&module_id, lifecycle_data).await.unwrap();

    MMFMod::wait_module_close(&module_id, None).await.unwrap();

    MMF::close().unwrap();

    println!("{:?}", std::env::current_dir().unwrap());
    println!("{:?}", std::env::current_exe().unwrap());

    // Read the generated log file.
    let log_file = fs::read_to_string(log_file_path).unwrap();

    // Validate the file content.
    assert!(log_file.contains("Mesh Module Framework Log File"));
    assert!(log_file.contains("LogID,Time,ModuleID"));
    assert!(log_file.contains("SPAWN"));
    assert!(log_file.contains("INIT"));
    assert!(log_file.contains("MESSAGE"));
    assert!(log_file.contains("Maximum log file size reached."));
    assert!(log_file.contains("File logging disabled."));
}
