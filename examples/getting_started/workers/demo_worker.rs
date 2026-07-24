//! A minimal MWF worker used by the Getting Started example.
//!
//! The implementation intentionally keeps the business logic simple so the
//! focus remains on the MWF worker lifecycle and messaging API.
//!
//! It demonstrates:
//! - worker registration;
//! - initialization through `on_init()`;
//! - message handling through `on_message()`;
//! - execution of both short and long blocking operations;
//! - graceful shutdown through `on_close()`.

use async_trait::async_trait;
use mesh_worker_framework::*;
use std::time::Duration;

/// Message identifiers handled by DemoWorker.
pub struct DemoMessages;

/// Defines the message identifiers.
impl DemoMessages {
    pub const QUICK_CALCULATION: &'static str = "QUICK_CALCULATION";
    pub const LONG_CALCULATION: &'static str = "LONG_CALCULATION";
}

/// Private data associated with a DemoWorker instance.
pub struct DemoWorker {
    worker_id: Option<WorkerID>,
    parent_id: Option<EntityID>,
}

/// MWF worker implementation for DemoWorker.
#[async_trait]
impl Worker for DemoWorker {
    fn worker_type_id() -> WorkerTypeID {
        "DemoWorker".into()
    }

    fn new() -> Self {
        Self { worker_id: None, parent_id: None }
    }

    async fn on_init(&mut self, worker_id: WorkerID, parent_id: EntityID, _init_data: MessageData) -> Result<(), WorkerError> {
        // Store identifiers for later use (for example to send messages or
        // spawn child workers from within the worker).
        self.worker_id = Some(worker_id);
        self.parent_id = Some(parent_id);

        Ok(())
    }

    async fn on_message(&mut self, message: WorkerMessage) -> Result<(), WorkerError> {
        match message.message_id.as_str() {
            DemoMessages::QUICK_CALCULATION => {
                // Small blocking work performed directly by the worker.
                std::thread::sleep(Duration::from_millis(10));
            }

            DemoMessages::LONG_CALCULATION => {
                // Long blocking work should be offloaded to a dedicated thread.
                MWFWrk::spawn_blocking(|| {
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

    async fn on_close(&mut self, _close_data: MessageData) -> Result<(), WorkerError> {
        Ok(())
    }
}
