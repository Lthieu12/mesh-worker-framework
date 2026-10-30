//! # Message Flow
//!
//! This example demonstrates how to wait for a processing flow that spans
//! multiple messages handled by the same worker.
//!
//! The application sends START_CALCULATION and waits for the complete flow.
//! The worker then sends PROCESS_CALCULATION to itself. The second message
//! completes the flow using workflow_end().
//!
//! MWF logs are enabled and displayed in the terminal to make message
//! processing and flow completion visible.
//!
//! Run with:
//!
//! cargo run --release --example message_flow_with_logging

use mwf::{LogMode::LogAll, *};

mod workers;
use workers::calculation_worker::{CalculationData, CalculationMessages, CalculationWorker};

fn main() -> Result<(), MWFError> {
    // Create the multi-threaded Tokio runtime required by MWF.
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        let main_entity = EntityID::new_external("MAIN");

        // Initialize MWF.
        MWF::init()?;

        // Register the worker type.
        MWF::register_worker_type::<CalculationWorker>()?;

        // Enable framework logs and display them in the terminal.
        MWFLog::set_log_mode(LogAll, None)?;
        MWFLog::set_log_terminal_enabled(true).await?;

        // Spawn and initialize the worker.
        let calculation_worker_id = MWFWrk::spawn_worker(&CalculationWorker::worker_type_id(), &main_entity)?;

        MWFWrk::init_worker(&calculation_worker_id, &()).await?;
        MWFWrk::wait_worker_init(&calculation_worker_id, None).await?;

        // Start the calculation and wait for the complete message flow.
        let calculation_data = CalculationData { value: 21 };

        let calculation_result = MWFWrk::send_message_and_wait(&main_entity, &calculation_worker_id, &CalculationMessages::START_CALCULATION, MessageWaitMode::WaitThisMessageFlow, &calculation_data, Some(5_000)).await;

        // Close the worker even if the message flow returned an error.
        MWFWrk::close_worker(&calculation_worker_id, &()).await?;
        MWFWrk::wait_worker_close(&calculation_worker_id, None).await?;

        // Shutdown MWF. This also stops terminal logging.
        MWF::close()?;

        // Propagate the message flow result after framework cleanup.
        calculation_result?;

        println!("Calculation flow completed successfully.");

        Ok(())
    })
}
