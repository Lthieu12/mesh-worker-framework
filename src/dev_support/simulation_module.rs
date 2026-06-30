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
use serde::{Serialize, Deserialize};

use crate::module::{Module, ModuleTypeID, ModuleID, EntityID, ModuleError};
use crate::message::{MessageID, ModuleMessage};
use crate::MMFMod;


#[derive(Debug, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum SimulationBehavior {

    DoNothing,

    Message_Wait{
        message_delay_ms: u64,
    },

    Message_SendAndWait {
        target_module_id: ModuleID,
        message_id: MessageID,
        timeout_ms: Option<u64>,
        message_delay_ms: u64,
    },

    Init_Fail,

    Close_Fail,

    Message_Fail,

    Message_Panic,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct SimulationModuleConfig {

    pub init_delay_ms: u64,

    pub close_delay_ms: u64,

    pub simulation_behavior: SimulationBehavior,

    pub wait_children_close: bool,

    pub wait_children_close_timeout_ms: Option<u64>,
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

    /// Artificial action applied.
    pub simulation_behavior: SimulationBehavior,

    /// Optional timeout used by on_close() to wait for
    /// all direct children to terminate.
    pub wait_children_close: bool,
    pub wait_children_close_timeout_ms: Option<u64>,
}


/// Defines the unique behavior simulated by this module.
///
/// A SimulationModule always simulates exactly one runtime
/// behavior. The configured behavior is applied to every
/// message received by the module.
///
/// This keeps integration tests deterministic and easy
/// to understand.
#[async_trait]
impl Module for SimulationModule {

    fn module_type_id() -> ModuleTypeID {
        "SimulationModule"
    }

    fn new() -> Self {

        Self {
            module_id: None,
            father_id: None,
            init_delay_ms: 0,
            close_delay_ms: 0,
            simulation_behavior: SimulationBehavior::DoNothing,
            wait_children_close: false,
            wait_children_close_timeout_ms: None,
        }
    }

    async fn on_init(&mut self, module_id: ModuleID, father_id: EntityID, lifecycle_data: Bytes) -> Result<(), ModuleError> {

        // Deserialize the simulation module configuration.
        let configuration: SimulationModuleConfig = bincode::deserialize(lifecycle_data.as_ref())?;

        self.init_delay_ms = configuration.init_delay_ms;
        self.close_delay_ms = configuration.close_delay_ms;
        self.simulation_behavior = configuration.simulation_behavior;
        self.wait_children_close = configuration.wait_children_close;
        self.wait_children_close_timeout_ms = configuration.wait_children_close_timeout_ms;
        
        // Store runtime identifiers provided by MMF.
        self.module_id = Some(module_id);
        self.father_id = Some(father_id);

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

    async fn on_message(&mut self, _message: ModuleMessage) -> Result<(), ModuleError> {
        
        match &self.simulation_behavior {

            SimulationBehavior::Message_SendAndWait {
                target_module_id,
                message_id,
                timeout_ms,
                message_delay_ms,
            } => {
                // Simulate message work.
                if *message_delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(*message_delay_ms)).await;
                }
                let module_id = self.module_id.as_ref().expect("module_id should be initialized");
                MMFMod::send_message_and_wait(&EntityID::Module(module_id.clone()), target_module_id, message_id, Bytes::new(), *timeout_ms).await.unwrap();
            }

            SimulationBehavior::Message_Wait {
                message_delay_ms,
            } => {
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

            _ => {} 
        }

        Ok(())
    }

    async fn on_close(&mut self, _lifecycle_data: Bytes) -> Result<(), ModuleError> {

        // Optionally wait until all direct children
        // have been removed from the Registry.
        if self.wait_children_close {

            let module_id = self.module_id.as_ref().expect("module_id should be initialized");

            MMFMod::wait_children_close(&EntityID::Module(module_id.clone()), self.wait_children_close_timeout_ms).await.unwrap();
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