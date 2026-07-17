//! A minimal MMF module used by the Getting Started example.
//!
//! The implementation intentionally keeps the business logic simple so the
//! focus remains on the MMF module lifecycle and messaging API.
//!
//! It demonstrates:
//! - module registration;
//! - initialization through `on_init()`;
//! - message handling through `on_message()`;
//! - execution of both short and long blocking operations;
//! - graceful shutdown through `on_close()`.

use async_trait::async_trait;
use bytes::Bytes;
use mesh_module_framework::*;
use std::time::Duration;

/// Message identifiers handled by DemoModule.
pub struct DemoMessages;

/// Defines the message identifiers.
impl DemoMessages {
    pub const QUICK_CALCULATION: &'static str = "QUICK_CALCULATION";
    pub const LONG_CALCULATION: &'static str = "LONG_CALCULATION";
}

/// Private data associated with a DemoModule instance.
pub struct DemoModule {
    module_id: Option<ModuleID>,
    father_id: Option<EntityID>,
}

/// MMF module implementation for DemoModule.
#[async_trait]
impl Module for DemoModule {
    fn module_type_id() -> ModuleTypeID {
        "DemoModule"
    }

    fn new() -> Self {
        Self { module_id: None, father_id: None }
    }

    async fn on_init(&mut self, module_id: ModuleID, father_id: EntityID, _lifecycle_data: Bytes) -> Result<(), ModuleError> {
        // Store identifiers for later use (for example to send messages or
        // spawn child modules from within the module).
        self.module_id = Some(module_id);
        self.father_id = Some(father_id);

        Ok(())
    }

    async fn on_message(&mut self, message: ModuleMessage) -> Result<(), ModuleError> {
        match message.message_id.as_str() {
            DemoMessages::QUICK_CALCULATION => {
                // Small blocking work performed directly by the module.
                std::thread::sleep(Duration::from_millis(10));
            }

            DemoMessages::LONG_CALCULATION => {
                // Long blocking work should be offloaded to a dedicated thread.
                MMFMod::spawn_blocking(|| {
                    std::thread::sleep(Duration::from_millis(1000));
                })
                .await?;
            }

            _ => {
                // Ignore unsupported messages.
            }
        }

        Ok(())
    }

    async fn on_close(&mut self, _lifecycle_data: Bytes) -> Result<(), ModuleError> {
        Ok(())
    }
}
