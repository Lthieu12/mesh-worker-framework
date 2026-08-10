//! # Getting Started
//!
//! This example demonstrates the complete lifecycle of a MWF worker with logs.
//!
//! It shows how to:
//! - initialize the framework;
//! - register a worker type;
//! - spawn and initialize a worker;
//! - exchange messages;
//! - gracefully shut down the worker and the framework.
//!
//! The `DemoWorker` implementation is intentionally minimal so the
//! focus remains on the MWF API rather than application logic.
//!
//! Run with:
//!
//! cargo run --release --example getting_started_with_log

use mesh_worker_framework::{LogMode::LogAll, *};

// DemoWorker and DemoMessages are defined in
// examples/getting_started/workers/demo_worker.rs.
mod workers;
use workers::demo_worker::{DemoMessages, DemoWorker};

// Tokio provides the async runtime used by MWF.
#[tokio::main]
async fn main() -> Result<(), MWFError> {
    let main_entity = EntityID::new_external("MAIN");

    // Initialize MWF.
    MWF::init()?;

    // Register available worker types.
    MWF::register_worker_type::<DemoWorker>()?;

    // Activate logs.
    MWFLog::set_log_mode(LogAll, None)?;
    MWFLog::set_log_terminal_enabled(true).await?;

    // Create the worker.
    let demo_worker_id = MWFWrk::spawn_worker(&DemoWorker::worker_type_id(), &main_entity)?;

    // Initialize it.
    MWFWrk::init_worker(&demo_worker_id, &()).await?;
    MWFWrk::wait_worker_init(&demo_worker_id, None).await?;

    // Send a few messages.
    MWFWrk::send_message(&main_entity, &demo_worker_id, &DemoMessages::QUICK_CALCULATION, &()).await?;
    MWFWrk::send_message(&main_entity, &demo_worker_id, &DemoMessages::LONG_CALCULATION, &()).await?;

    // Close the worker.
    MWFWrk::close_worker(&demo_worker_id, &()).await?;
    MWFWrk::wait_worker_close(&demo_worker_id, None).await?;

    // Shutdown.
    MWF::close()?;

    Ok(())
}
