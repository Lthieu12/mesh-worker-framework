//! Worker demonstrating message flow propagation between messages.

use async_trait::async_trait;
use mwf::*;

/// Data exchanged during the calculation.
#[derive(Clone)]
pub struct CalculationData {
    pub value: i64,
}

/// Message identifiers handled by CalculationWorker.
pub struct CalculationMessages;

impl CalculationMessages {
    pub const START_CALCULATION: &'static str = "START_CALCULATION";
    pub const PROCESS_CALCULATION: &'static str = "PROCESS_CALCULATION";
}

/// Worker demonstrating a multi-message calculation flow.
pub struct CalculationWorker;

#[async_trait]
impl Worker for CalculationWorker {
    fn worker_type_id() -> WorkerTypeID {
        "CalculationWorker".into()
    }

    fn new() -> Self {
        Self
    }

    async fn on_init(&mut self, _init_data: MessageData) -> Result<(), WorkerError> {
        Ok(())
    }

    async fn on_message(&mut self, message: WorkerMessage) -> Result<(), WorkerError> {
        match message.message_id.as_str() {
            CalculationMessages::START_CALCULATION => {
                // Recover the expected payload type. MessageData uses typed
                // Rust values; this is a downcast, not JSON deserialization.
                let calculation_data = match message.message_data.into::<CalculationData>() {
                    Ok(data) => data,
                    Err(error) => {
                        // Explicitly complete the flow when the payload type is unexpected.
                        MWFWrk::workflow_end(Err(WorkerError::from(error)))?;
                        return Ok(());
                    }
                };

                let worker_id = MWFWrk::get_current_worker_id()?;

                // Send the next message to the same worker.
                // MWF automatically propagates the active message flow.
                if let Err(error) = MWFWrk::send_message(&worker_id.as_entity_id(), &worker_id, &CalculationMessages::PROCESS_CALCULATION, &calculation_data).await {
                    // Complete the flow if the next message cannot be queued.
                    MWFWrk::workflow_end(Err(WorkerError::from(error)))?;
                }
            }

            CalculationMessages::PROCESS_CALCULATION => {
                let calculation_data = match message.message_data.into::<CalculationData>() {
                    Ok(data) => data,
                    Err(error) => {
                        // Do not leave the caller waiting if payload recovery fails.
                        MWFWrk::workflow_end(Err(WorkerError::from(error)))?;
                        return Ok(());
                    }
                };

                // Keep the business result separate from the callback result.
                let calculation_result = Self::process_calculation(calculation_data).await;

                // Forward the business result to the application waiting for the flow.
                MWFWrk::workflow_end(calculation_result)?;
            }

            _ => {}
        }

        Ok(())
    }

    async fn on_close(&mut self, _close_data: MessageData) -> Result<(), WorkerError> {
        Ok(())
    }
}

impl CalculationWorker {
    /// Execute the application-level calculation.
    async fn process_calculation(calculation_data: CalculationData) -> Result<(), WorkerError> {
        let _result = calculation_data.value.checked_mul(2).ok_or_else(|| WorkerError::msg("Calculation overflow"))?;

        Ok(())
    }
}
