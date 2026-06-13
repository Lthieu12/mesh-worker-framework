//! MMF module used by integration tests to record message
//! execution order and measure wait durations.
#![allow(dead_code)]

use async_trait::async_trait;
use bytes::Bytes;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use mesh_module_framework::*;

pub const MESSAGE_CHAIN: &str = "CHAIN";
pub const MESSAGE_PRIORITY_1: &str = "PRIORITY_1";
pub const MESSAGE_PRIORITY_2: &str = "PRIORITY_2";
pub const MESSAGE_NORMAL_1: &str = "NORMAL_1";
pub const MESSAGE_NORMAL_2: &str = "NORMAL_2";

pub struct MessageRecorderModule {

    /// Runtime ModuleID assigned by MMF during on_init().
    module_id: Option<ModuleID>,

    /// Runtime FatherID assigned by MMF during on_init().
    father_id: Option<EntityID>,

    /// Target module used by CHAIN messages.
    target_module_id: ModuleID,

    /// Shared execution log.
    pub execution_log: Arc<Mutex<Vec<String>>>,

    /// Shared wait duration storage.
    pub wait_duration_ms: Arc<Mutex<Option<u128>>>,
}

impl MessageRecorderModule {

    /// Spawn and initialize a MessageRecorderModule.
    pub async fn spawn(
        father_id: &EntityID,
        target_module_id: ModuleID,
        execution_log: Arc<Mutex<Vec<String>>>,
        wait_duration_ms: Arc<Mutex<Option<u128>>>,
    ) -> Result<ModuleID, FrameworkError> {

        let module_id =
            MMF::spawn_module(
                MessageRecorderModule {
                    module_id: None,
                    father_id: None,
                    target_module_id,
                    execution_log,
                    wait_duration_ms,
                },
                father_id,
            )?;

        Ok(module_id)
    }
}

#[async_trait]
impl Module for MessageRecorderModule {

    async fn on_init(
        &mut self,
        module_id: &ModuleID,
        father_id: &EntityID,
    ) {

        self.module_id = Some(module_id.clone());
        self.father_id = Some(father_id.clone());
    }

    async fn on_message(
        &mut self,
        message: FrameworkMessage,
    ) {

        match message.message_id.as_str() {

            MESSAGE_CHAIN => {

                let module_id =
                    self.module_id
                        .as_ref()
                        .expect("module_id should be initialized");

                let start_time = Instant::now();

                MMF::send_message_and_wait_reply(
                    &EntityID::Module(module_id.clone()),
                    &self.target_module_id,
                    &message.message_id,
                    Bytes::new(),
                    Some(2000),
                )
                .await
                .unwrap();

                let elapsed_ms =
                    start_time.elapsed().as_millis();

                let mut duration_guard =
                    self.wait_duration_ms.lock().unwrap();

                *duration_guard = Some(elapsed_ms);

                drop(duration_guard);

                let mut log_guard =
                    self.execution_log.lock().unwrap();

                log_guard.push(MESSAGE_CHAIN.to_string());

                drop(log_guard);
            }

            MESSAGE_PRIORITY_1 => {

                let mut log_guard =
                    self.execution_log.lock().unwrap();

                log_guard.push(MESSAGE_PRIORITY_1.to_string());

                drop(log_guard);
            }

            MESSAGE_PRIORITY_2 => {

                let mut log_guard =
                    self.execution_log.lock().unwrap();

                log_guard.push(MESSAGE_PRIORITY_2.to_string());

                drop(log_guard);
            }

            MESSAGE_NORMAL_1 => {

                let mut log_guard =
                    self.execution_log.lock().unwrap();

                log_guard.push(MESSAGE_NORMAL_1.to_string());

                drop(log_guard);
            }

            MESSAGE_NORMAL_2 => {

                let mut log_guard =
                    self.execution_log.lock().unwrap();

                log_guard.push(MESSAGE_NORMAL_2.to_string());

                drop(log_guard);
            }

            _ => {}
        }
    }

    async fn on_close(
        &mut self,
    ) {
    }
}