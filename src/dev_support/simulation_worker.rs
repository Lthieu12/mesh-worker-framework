//! Generic MWF worker used by integration tests.
//!
//! This configurable worker can simulate various runtime
//! behaviors without requiring dedicated test workers.
//!
//! Supported behaviors include:
//! - Initialization delays.
//! - Message processing delays.
//! - Chained send_message_and_wait() calls.
//! - Shutdown delays.
//! - Parent waiting for child shutdown.
//! - ...
//!
//! Runtime behavior is configured through
//! SimulationMessageBehavior.
//!
//! Additional test behaviors can be added over time.

#![allow(dead_code)]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::MWFWrk;
use crate::mwf_wrk::traits::Worker;
use crate::mwf_wrk::types::{EntityID, WorkerError, WorkerID, WorkerTypeID};
use crate::mwf_wrk::types::{MessageData, MessageID, MessageWaitMode, WorkerMessage};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum SimulationBehavior {
    DoNothing,

    Message_Wait { message_delay_ms: u64 },

    Message_SendAndWait { target_worker_id: WorkerID, message_id: MessageID, timeout_ms: Option<u64>, message_delay_ms: u64 },

    Message_SendPriorityAndWait { target_worker_id: WorkerID, message_id: MessageID, timeout_ms: Option<u64>, message_delay_ms: u64 },

    Init_Fail,

    Close_Fail,

    Message_Fail,

    Message_Panic,

    Message_CheckMyContext { expected_worker_id: WorkerID, expected_parent_id: EntityID },

    MessageFlow_End { message_delay_ms: u64 },

    MessageFlow_EndFail { message_delay_ms: u64 },

    MessageFlow_EndChained { message_delay_ms: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationWorkerConfig {
    pub init_delay_ms: u64,

    pub close_delay_ms: u64,

    pub simulation_behavior: SimulationBehavior,

    pub wait_children_close: bool,

    pub wait_children_close_timeout_ms: Option<u64>,
}

pub struct SimulationWorker {
    /// Runtime WorkerID assigned by MWF during on_init().
    worker_id: Option<WorkerID>,

    /// Runtime ParentID assigned by MWF during on_init().
    parent_id: Option<EntityID>,

    /// Artificial delay applied during on_init().
    pub init_delay_ms: u64,

    /// Artificial delay applied during on_close().
    pub close_delay_ms: u64,

    /// Artificial action applied.
    pub simulation_behavior: SimulationBehavior,

    /// Optional timeout used by on_close() to wait for
    /// all direct children to terminate.
    pub wait_children_close: bool,
    pub wait_children_close_timeout_ms: Option<u64>,

    /// Number of messages processed by the worker
    /// during on_message(MessageFlow_EndChained)
    message_count: usize,
}

/// Defines the unique behavior simulated by this worker.
///
/// A SimulationWorker always simulates exactly one runtime
/// behavior. The configured behavior is applied to every
/// message received by the worker.
///
/// This keeps integration tests deterministic and easy
/// to understand.
#[async_trait]
impl Worker for SimulationWorker {
    fn worker_type_id() -> WorkerTypeID {
        "SimulationWorker".into()
    }

    fn new() -> Self {
        Self { worker_id: None, parent_id: None, init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::DoNothing, wait_children_close: false, wait_children_close_timeout_ms: None, message_count: 0 }
    }

    async fn on_init(&mut self, init_data: MessageData) -> Result<(), WorkerError> {
        // Deserialize the simulation worker configuration.
        let configuration = init_data.into::<SimulationWorkerConfig>()?;

        self.init_delay_ms = configuration.init_delay_ms;
        self.close_delay_ms = configuration.close_delay_ms;
        self.simulation_behavior = configuration.simulation_behavior;
        self.wait_children_close = configuration.wait_children_close;
        self.wait_children_close_timeout_ms = configuration.wait_children_close_timeout_ms;
        self.message_count = 0;

        // Store runtime identifiers from the current worker execution context.
        self.worker_id = Some(MWFWrk::get_current_worker_id()?);
        self.parent_id = Some(MWFWrk::get_current_parent_id()?);

        // Simulate init work.
        if self.init_delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.init_delay_ms)).await;
        }

        match &self.simulation_behavior {
            SimulationBehavior::Init_Fail => {
                return Err(anyhow::anyhow!("Simulation init failure"));
            }

            _ => {}
        }
        Ok(())
    }

    async fn on_message(&mut self, _message: WorkerMessage) -> Result<(), WorkerError> {
        match &self.simulation_behavior {
            SimulationBehavior::Message_SendAndWait { target_worker_id, message_id, timeout_ms, message_delay_ms } => {
                // Simulate message work.
                if *message_delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(*message_delay_ms)).await;
                }
                let worker_id = self.worker_id.as_ref().expect("worker_id should be initialized");
                MWFWrk::send_message_and_wait(&EntityID::new_worker(worker_id.clone()), target_worker_id, message_id, MessageWaitMode::WaitThisMessage, &(), *timeout_ms).await?;
            }

            SimulationBehavior::Message_SendPriorityAndWait { target_worker_id, message_id, timeout_ms, message_delay_ms } => {
                // Simulate message processing work before sending.
                if *message_delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(*message_delay_ms)).await;
                }

                let worker_id = self.worker_id.as_ref().expect("worker_id should be initialized");

                // Send a priority message and wait for its completion.
                MWFWrk::send_priority_message_and_wait(&EntityID::new_worker(worker_id.clone()), target_worker_id, message_id, MessageWaitMode::WaitThisMessage, &(), *timeout_ms).await?;
            }

            SimulationBehavior::Message_Wait { message_delay_ms } => {
                // Simulate message work.
                if *message_delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(*message_delay_ms)).await;
                }
            }

            SimulationBehavior::Message_Fail => {
                return Err(anyhow::anyhow!("Simulation message failure"));
            }

            SimulationBehavior::Message_Panic => {
                panic!("Simulation message panic");
            }

            SimulationBehavior::Message_CheckMyContext { expected_worker_id, expected_parent_id } => {
                // Retrieve identifiers from the current worker execution context.
                let current_worker_id = MWFWrk::get_current_worker_id()?;
                let current_parent_id = MWFWrk::get_current_parent_id()?;

                // Validate the current worker ID.
                if &current_worker_id != expected_worker_id {
                    return Err(anyhow::anyhow!("Unexpected current WorkerID: expected '{expected_worker_id}', got '{current_worker_id}'"));
                }

                // Validate the current parent ID.
                if &current_parent_id != expected_parent_id {
                    return Err(anyhow::anyhow!("Unexpected current parent ID: expected '{expected_parent_id}', got '{current_parent_id}'"));
                }
            }

            SimulationBehavior::MessageFlow_End { message_delay_ms } => {
                // Simulate message work before explicitly completing the message flow.
                if *message_delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(*message_delay_ms)).await;
                }

                MWFWrk::message_flow_end(Ok(()))?;
            }

            SimulationBehavior::MessageFlow_EndFail { message_delay_ms } => {
                // Simulate message work before explicitly failing the flow.
                if *message_delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(*message_delay_ms)).await;
                }

                MWFWrk::message_flow_end(Err(anyhow::anyhow!("Simulation message failure")))?;
            }

            SimulationBehavior::MessageFlow_EndChained { message_delay_ms } => {
                // Simulate message processing work.
                if *message_delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(*message_delay_ms)).await;
                }

                self.message_count += 1;

                if self.message_count < 3 {
                    let worker_id = self.worker_id.as_ref().expect("worker_id should be initialized");

                    // Send the next message to the same worker.
                    MWFWrk::send_message(&EntityID::new_worker(worker_id.clone()), worker_id, &MessageID::new("MESSAGE_FLOW_END_CHAINED"), &()).await?;
                } else {
                    // Complete the message flow after the third message.
                    MWFWrk::message_flow_end(Ok(()))?;
                }
            }

            _ => {}
        }

        Ok(())
    }

    async fn on_close(&mut self, _close_data: MessageData) -> Result<(), WorkerError> {
        // Optionally wait until all direct children
        // have been removed from the Registry.
        if self.wait_children_close {
            let worker_id = self.worker_id.as_ref().expect("worker_id should be initialized");

            MWFWrk::wait_children_close(&worker_id.as_entity_id(), self.wait_children_close_timeout_ms).await.unwrap();
        }

        // Simulate shutdown work.
        if self.close_delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.close_delay_ms)).await;
        }

        match &self.simulation_behavior {
            SimulationBehavior::Close_Fail => {
                return Err(anyhow::anyhow!("Simulation close failure"));
            }

            _ => {}
        }

        Ok(())
    }
}
