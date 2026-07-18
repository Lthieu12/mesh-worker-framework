//! # Getting Started
//!
//! This example demonstrates the complete lifecycle of a MMF worker.
//!
//! It shows how to:
//! - initialize the framework;
//! - register a worker type;
//! - spawn and initialize a worker;
//! - exchange messages;
//! - gracefully shut down the worker and the framework.
//!
//! The `DemoWorker` implementation is intentionally minimal so the
//! focus remains on the MMF API rather than application logic.
//!
//! Run with:
//!
//! cargo run --release --example getting_started

use bytes::Bytes;

use mesh_worker_framework::*;

// DemoWorker and DemoMessages are defined in
// examples/getting_started/workers/demo_worker.rs.
mod modules;
use modules::demo_worker::{DemoMessages, DemoWorker};

// Tokio provides the async runtime used by MMF.
#[tokio::main]
async fn main() -> Result<(), MMFError> {
    let main_entity = EntityID::new_external("MAIN");

    // Initialize MMF.
    MMF::init()?;

    // Register available worker types.
    MMF::register_worker_type::<DemoWorker>()?;

    // Create the worker.
    let demo_worker_id = MMFWrk::spawn_worker("DemoWorker", &main_entity)?;

    // Initialize it.
    MMFWrk::init_worker(&demo_worker_id, Bytes::new()).await?;
    MMFWrk::wait_worker_init(&demo_worker_id, None).await?;

    // Send a few messages.
    MMFWrk::send_message(&main_entity, &demo_worker_id, DemoMessages::QUICK_CALCULATION, Bytes::new()).await?;
    MMFWrk::send_message(&main_entity, &demo_worker_id, DemoMessages::LONG_CALCULATION, Bytes::new()).await?;

    // Close the worker.
    MMFWrk::close_worker(&demo_worker_id, Bytes::new()).await?;
    MMFWrk::wait_worker_close(&demo_worker_id, None).await?;

    // Shutdown.
    MMF::close()?;

    Ok(())
}
