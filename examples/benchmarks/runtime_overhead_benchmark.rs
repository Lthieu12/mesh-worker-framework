//! MWF runtime overhead benchmark.
//!
//! Run with:
//!
//! cargo run --release --example runtime_overhead_benchmark
//! cargo run --release --example runtime_overhead_benchmark -- 100
//! cargo run --release --example runtime_overhead_benchmark -- 5000
//!
//! If no argument is provided, 1000 workers are used.

use std::time::Instant;

use futures::future::join_all;

use mwf::dev_support::simulation_worker::{SimulationBehavior, SimulationWorker, SimulationWorkerConfig};
use mwf::*;

const DEFAULT_WORKER_COUNT: usize = 1000;
const TIMEOUT_MS: u64 = 5000;

#[tokio::main]
async fn main() -> Result<(), MWFError> {
    let worker_count = std::env::args().nth(1).and_then(|arg| arg.parse::<usize>().ok()).unwrap_or(DEFAULT_WORKER_COUNT);

    MWF::init()?;

    MWF::register_worker_type::<SimulationWorker>()?;

    MWFLog::set_log_mode(LogMode::NoLog, None)?;

    let configuration = SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::DoNothing, wait_children_close: false, wait_children_close_timeout_ms: None };

    //--------------------------------------------------
    // Spawn
    //--------------------------------------------------

    let total_timer = Instant::now();

    let timer = Instant::now();

    let mut worker_ids = Vec::with_capacity(worker_count);

    for _ in 0..worker_count {
        let worker_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &EntityID::External("MAIN".into()))?;

        worker_ids.push(worker_id);
    }

    let spawn_time = timer.elapsed();

    //--------------------------------------------------
    // Init
    //--------------------------------------------------

    let timer = Instant::now();

    for worker_id in &worker_ids {
        MWFWrk::init_worker(worker_id, &configuration).await?;
    }

    MWFWrk::wait_worker_list_init(&worker_ids, Some(TIMEOUT_MS)).await?;

    let init_time = timer.elapsed();

    //--------------------------------------------------
    // Parallel SendAndWait
    //--------------------------------------------------

    let timer = Instant::now();

    let sender_id = EntityID::new_external("MAIN");
    let message_id = MessageID::new("PING");

    let futures = worker_ids.iter().map(|worker_id| MWFWrk::send_message_and_wait(&sender_id, worker_id, &message_id, MessageWaitMode::WaitThisMessage, &(), Some(TIMEOUT_MS)));

    let results = join_all(futures).await;

    for result in results {
        result.unwrap();
    }

    let messaging_time = timer.elapsed();

    //--------------------------------------------------
    // Close
    //--------------------------------------------------

    let timer = Instant::now();

    for worker_id in &worker_ids {
        MWFWrk::close_worker(worker_id, &()).await?;
    }

    MWFWrk::wait_children_close(&EntityID::new_external("MAIN"), Some(TIMEOUT_MS)).await?;

    let close_time = timer.elapsed();

    //--------------------------------------------------
    // Statistics
    //--------------------------------------------------

    let total_time = total_timer.elapsed();

    let throughput = worker_count as f64 / messaging_time.as_secs_f64();

    println!();
    println!("=====================================================");
    println!("            MWF runtime overhead benchmark");
    println!("=====================================================");
    println!("Workers : {}", worker_count);
    println!();

    println!("Spawn                   : {:>8.3?} ({:.2} µs/worker)", spawn_time, spawn_time.as_secs_f64() * 1_000_000.0 / worker_count as f64,);

    println!("Init                    : {:>8.3?} ({:.2} µs/worker)", init_time, init_time.as_secs_f64() * 1_000_000.0 / worker_count as f64,);

    println!("SendAndWait (parallel)  : {:>8.3?} ({:.2} µs/message)", messaging_time, messaging_time.as_secs_f64() * 1_000_000.0 / worker_count as f64,);

    println!("Close                   : {:>8.3?} ({:.2} µs/worker)", close_time, close_time.as_secs_f64() * 1_000_000.0 / worker_count as f64,);

    println!();

    println!("Total                   : {:>8.3?}", total_time);
    println!("Throughput              : {:.0} msg/s", throughput);
    println!("=====================================================");
    println!();

    MWF::close()?;

    Ok(())
}
