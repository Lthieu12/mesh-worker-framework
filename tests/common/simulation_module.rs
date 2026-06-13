//! Generic MMF module used by integration tests.
//!
//! This configurable module can simulate various runtime
//! behaviors without requiring dedicated test modules.
//!
//! Supported behaviors include:
//! - Initialization delays.
//! - Message processing delays.
//! - Chained send_message_and_wait_reply() calls.
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
use std::time::Duration;
use bytes::Bytes;

use mesh_module_framework::*;


pub enum SimulationMessageBehavior {

    DoNothing,

    Wait{
        message_delay_ms: u64,
    },

    SendAndWaitReply {
        target_module_id: ModuleID,
        message_id: MessageID,
        timeout_ms: Option<u64>,
        message_delay_ms: u64,
    },

}

pub struct SimulationModule {

    /// Runtime ModuleID assigned by MMF during on_init().
    module_id: Option<ModuleID>,

    /// Runtime FatherID assigned by MMF during on_init().
    father_id: Option<EntityID>,

    /// Artificial delay applied during on_init().
    pub init_delay_ms: u64,

    /// Artificial delay applied during on_close().
    pub close_delay_ms: u64,

    /// Artificial action applied during on_message().
    pub message_behavior: SimulationMessageBehavior,

    /// Optional timeout used by on_close() to wait for
    /// all direct children to terminate.
    pub wait_children_close: bool,
    pub wait_children_close_timeout_ms: Option<u64>,
}

impl SimulationModule {

    /// Spawn and initialize a SimulationModule.
    ///
    /// This helper is intended for integration tests and
    /// returns only when the module has reached the
    /// Initialized state.
    pub async fn spawn(father_id: &EntityID, init_delay_ms: u64, close_delay_ms: u64, message_behavior: SimulationMessageBehavior, wait_children_close: bool, wait_children_close_timeout_ms: Option<u64>) -> Result<ModuleID, FrameworkError> {

        let module_id =
            MMF::spawn_module(
                SimulationModule {
                    module_id: None,
                    father_id: None,
                    init_delay_ms,
                    close_delay_ms,
                    message_behavior,
                    wait_children_close,
                    wait_children_close_timeout_ms,

                },
                father_id,
            )?;

        Ok(module_id)
    }
}

#[async_trait]
impl Module for SimulationModule {

    async fn on_init(&mut self, module_id: &ModuleID, father_id: &EntityID) {

        // Store runtime identifiers provided by MMF.
        self.module_id = Some(module_id.clone());
        self.father_id = Some(father_id.clone());

        // Simulate init work.
        tokio::time::sleep(Duration::from_millis(self.init_delay_ms)).await;
    }

    async fn on_message(&mut self, _message: FrameworkMessage) {
        
        match &self.message_behavior {

            SimulationMessageBehavior::DoNothing => {}

                SimulationMessageBehavior::SendAndWaitReply {
                    target_module_id,
                    message_id,
                    timeout_ms,
                    message_delay_ms,
                } => {
                    tokio::time::sleep(Duration::from_millis(*message_delay_ms)).await;
                    let module_id = self.module_id.as_ref().expect("module_id should be initialized");
                    MMF::send_message_and_wait_reply(&EntityID::Module(module_id.clone()), target_module_id, message_id, Bytes::new(), *timeout_ms).await.unwrap();
                }

            SimulationMessageBehavior::Wait {
                message_delay_ms,
            } => {
                // Simulate message work.
                tokio::time::sleep(Duration::from_millis(*message_delay_ms)).await;
            }


        }
        
    }

    async fn on_close(&mut self) {

        // Optionally wait until all direct children
        // have been removed from the Registry.
        if self.wait_children_close {

            let module_id = self.module_id.as_ref().expect("module_id should be initialized");

            MMF::wait_children_close(&EntityID::Module(module_id.clone()), self.wait_children_close_timeout_ms).await.unwrap();
        }

        // Simulate shutdown work.
        tokio::time::sleep(Duration::from_millis(self.close_delay_ms)).await;
    }
}