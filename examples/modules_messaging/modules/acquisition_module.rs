use async_trait::async_trait;
use bytes::Bytes;
use mesh_worker_framework::*;
use std::time::Duration;

/// Message identifiers handled by DemoWorker.
pub struct AcquisitionWorkerMessages;

/// Defines the message identifiers.
impl AcquisitionWorkerMessages {
    pub const START_worker_WRKFLW: &'static str = "CONFIGURE_worker";
    pub const LONG_CALCULATION: &'static str = "LONG_CALCULATION";
}

/// Private data associated with a DemoWorker instance.
pub struct AcquisitionWorker {
    worker_id: Option<WorkerID>,
    parent_id: Option<EntityID>,
}

/// MMF worker implementation for DemoWorker.
#[async_trait]
impl Worker for AcquisitionWorker {
    fn worker_type_id() -> WorkerTypeID {
        "DemoWorker"
    }

    fn new() -> Self {
        Self { worker_id: None, parent_id: None }
    }

    async fn on_init(&mut self, worker_id: WorkerID, parent_id: EntityID, _lifecycle_data: Bytes) -> Result<(), WorkerError> {
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
                MMFWrk::spawn_blocking(|| {
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

    async fn on_close(&mut self, _lifecycle_data: Bytes) -> Result<(), WorkerError> {
        Ok(())
    }
}