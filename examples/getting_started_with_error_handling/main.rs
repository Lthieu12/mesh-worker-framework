//! # Getting Started with Error Handling (Synchronous Main)
//!
//! This example demonstrates how to report application errors through MWFErr.
//!
//! It shows how to:
//! - create a multi-threaded Tokio runtime explicitly;
//! - initialize MWF;
//! - activate terminal error logging;
//! - report application errors with MWFErr;
//! - gracefully shut down the framework.
//!
//! This example also demonstrates how to use MWF from a synchronous
//! application by creating a multi-threaded Tokio runtime explicitly.
//!
//! Run with:
//!
//! cargo run --release --example getting_started_with_error_handling

use anyhow::{Context, Result};
use mwf::*;

/// Generate an application error for the given attempt.
fn generate_error(attempt_index: u32) -> Result<()> {
    Err(anyhow::anyhow!("Operation failed on attempt {}", attempt_index)).context("Failed to execute application operation")
}

fn main() -> Result<(), MWFError> {
    // Create the multi-threaded Tokio runtime required by MWF.
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        let application_entity = EntityID::new_external("Application");

        // Initialize MWF.
        MWF::init()?;

        // Activate terminal error logging.
        MWFErr::set_error_terminal_enabled(true).await?;

        // Generate and report multiple application errors.
        for attempt_index in 1..=3 {
            if let Err(error) = generate_error(attempt_index) {
                MWFErr::add_error(&application_entity, &error)?;
            }
        }

        // Shutdown.
        MWF::close()?;

        Ok(())
    })
}
