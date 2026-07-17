//! MMF runtime overhead benchmark.
//!
//! Run with:
//!
//! cargo run --release --example runtime_overhead_benchmark
//! cargo run --release --example runtime_overhead_benchmark -- 100
//! cargo run --release --example runtime_overhead_benchmark -- 5000
//!
//! If no argument is provided, 1000 modules are used.

use std::time::Instant;

use bytes::Bytes;
use futures::future::join_all;

use mesh_module_framework::dev_support::simulation_module::{SimulationBehavior, SimulationModule, SimulationModuleConfig};
use mesh_module_framework::*;

const DEFAULT_MODULE_COUNT: usize = 1000;
const TIMEOUT_MS: u64 = 5000;

#[tokio::main]
async fn main() {
    let module_count = std::env::args().nth(1).and_then(|arg| arg.parse::<usize>().ok()).unwrap_or(DEFAULT_MODULE_COUNT);

    MMF::init().unwrap();

    MMF::register_module_type::<SimulationModule>().unwrap();

    MMFLog::set_log_mode(LogMode::NoLog, None).unwrap();

    let configuration = SimulationModuleConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::DoNothing, wait_children_close: false, wait_children_close_timeout_ms: None };

    let lifecycle_data = Bytes::from(bincode::serialize(&configuration).unwrap());

    //--------------------------------------------------
    // Spawn
    //--------------------------------------------------

    let total_timer = Instant::now();

    let timer = Instant::now();

    let mut module_ids = Vec::with_capacity(module_count);

    for _ in 0..module_count {
        let module_id = MMFMod::spawn_module(SimulationModule::module_type_id(), &EntityID::External("MAIN".into())).unwrap();

        module_ids.push(module_id);
    }

    let spawn_time = timer.elapsed();

    //--------------------------------------------------
    // Init
    //--------------------------------------------------

    let timer = Instant::now();

    for module_id in &module_ids {
        MMFMod::init_module(module_id, lifecycle_data.clone()).await.unwrap();
    }

    MMFMod::wait_module_list_init(&module_ids, Some(TIMEOUT_MS)).await.unwrap();

    let init_time = timer.elapsed();

    //--------------------------------------------------
    // Parallel SendAndWait
    //--------------------------------------------------

    let timer = Instant::now();

    let sender_id = EntityID::new_external("MAIN");
    let message_id = MessageID::new("PING");

    let futures = module_ids.iter().map(|module_id| MMFMod::send_message_and_wait(&sender_id, module_id, &message_id, Bytes::new(), Some(TIMEOUT_MS)));

    let results = join_all(futures).await;

    for result in results {
        result.unwrap();
    }

    let messaging_time = timer.elapsed();

    //--------------------------------------------------
    // Close
    //--------------------------------------------------

    let timer = Instant::now();

    for module_id in &module_ids {
        MMFMod::close_module(module_id, Bytes::new()).await.unwrap();
    }

    MMFMod::wait_children_close(&EntityID::new_external("MAIN"), Some(TIMEOUT_MS)).await.unwrap();

    let close_time = timer.elapsed();

    //--------------------------------------------------
    // Statistics
    //--------------------------------------------------

    let total_time = total_timer.elapsed();

    let throughput = module_count as f64 / messaging_time.as_secs_f64();

    println!();
    println!("=====================================================");
    println!("            MMF runtime overhead benchmark");
    println!("=====================================================");
    println!("Modules : {}", module_count);
    println!();

    println!("Spawn                   : {:>8.3?} ({:.2} µs/module)", spawn_time, spawn_time.as_secs_f64() * 1_000_000.0 / module_count as f64,);

    println!("Init                    : {:>8.3?} ({:.2} µs/module)", init_time, init_time.as_secs_f64() * 1_000_000.0 / module_count as f64,);

    println!("SendAndWait (parallel)  : {:>8.3?} ({:.2} µs/message)", messaging_time, messaging_time.as_secs_f64() * 1_000_000.0 / module_count as f64,);

    println!("Close                   : {:>8.3?} ({:.2} µs/module)", close_time, close_time.as_secs_f64() * 1_000_000.0 / module_count as f64,);

    println!();

    println!("Total                   : {:>8.3?}", total_time);
    println!("Throughput              : {:.0} msg/s", throughput);
    println!("=====================================================");
    println!();

    MMF::close().unwrap();
}
