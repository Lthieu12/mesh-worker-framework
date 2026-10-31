//! Tests that `MWFWrk::spawn_blocking()` preserves the MWF worker context
//! even when the originating worker is aborted and MWF is reinitialized.
//!
//! Test steps:
//! 1. Initialize MWF and start a worker whose message callback launches blocking work.
//! 2. Wait until the blocking closure starts and blocks on a synchronization channel.
//! 3. Abort the originating worker and close the first MWF instance while the closure is still running.
//! 4. Reinitialize MWF and create a new worker that can receive messages.
//! 5. Release the old blocking closure and let it attempt to send a message to the new worker.
//! 6. Verify that the send operation returns `Worker_NotFound`, proving that the original worker context
//!    was preserved instead of being treated as an external call.
//! 7. Close the new worker and the second MWF instance.
use serial_test::serial;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use async_trait::async_trait;

use mwf::{EntityID, MWF, MWFConfiguration, MWFError, MWFWrk, MessageData, Worker, WorkerError, WorkerID, WorkerMessage, WorkerTypeID};

// Data used to synchronize the test with the blocking closure.
#[derive(Clone)]
struct BlockingTestPayload {
    started_sender: mpsc::Sender<()>,
    target_worker_receiver: Arc<Mutex<mpsc::Receiver<WorkerID>>>,
    result_sender: mpsc::Sender<bool>,
}

// Worker whose message callback starts a blocking operation.
struct BlockingWorker;

#[async_trait]
impl Worker for BlockingWorker {
    fn worker_type_id() -> WorkerTypeID {
        WorkerTypeID::new("BlockingWorkerContextTest")
    }

    fn new() -> Self {
        Self
    }

    async fn on_init(&mut self, _init_data: MessageData) -> Result<(), WorkerError> {
        Ok(())
    }

    async fn on_message(&mut self, message: WorkerMessage) -> Result<(), WorkerError> {
        let payload = message.message_data.into::<BlockingTestPayload>()?;

        // Capture the runtime handle before entering the blocking closure.
        let runtime_handle = tokio::runtime::Handle::current();

        // Start blocking work while preserving the MWF worker context.
        MWFWrk::spawn_blocking(move || {
            // Notify the test that the blocking closure is running.
            payload.started_sender.send(()).expect("Failed to notify test");

            // Wait until the test provides a worker from the new MWF instance.
            let receiver_id = payload.target_worker_receiver.lock().expect("Target worker receiver lock poisoned").recv().expect("Failed to receive target worker ID");

            // This should fail because the original worker no longer exists.
            // The external sender ID ensures that only context propagation can reject it.
            let send_result = runtime_handle.block_on(MWFWrk::send_message(&EntityID::new_external("blocking-test-sender"), &receiver_id, &"message-after-reinitialization", &()));

            payload.result_sender.send(matches!(send_result, Err(MWFError::Worker_NotFound))).expect("Failed to send test result");
        })
        .await?;

        Ok(())
    }

    async fn on_close(&mut self, _close_data: MessageData) -> Result<(), WorkerError> {
        Ok(())
    }
}

fn test_configuration() -> MWFConfiguration {
    let mut configuration = MWFConfiguration::default();

    // Prevent the test from creating a log file.
    configuration.log_file_path = None;

    configuration
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn spawn_blocking_preserves_worker_context_after_abort_and_reinit() {
    // Initialize the first MWF instance.
    MWF::init_with_configuration(test_configuration()).unwrap();
    MWF::register_worker_type::<BlockingWorker>().unwrap();

    let parent_id = EntityID::new_external("test-parent");

    let sender_worker_id = MWFWrk::spawn_worker(&BlockingWorker::worker_type_id(), &parent_id).unwrap();

    MWFWrk::init_worker(&sender_worker_id, &()).await.unwrap();
    MWFWrk::wait_worker_init(&sender_worker_id, Some(5_000)).await.unwrap();

    // Prepare synchronization channels.
    let (started_sender, started_receiver) = mpsc::channel();
    let (target_worker_sender, target_worker_receiver) = mpsc::channel();
    let (result_sender, result_receiver) = mpsc::channel();

    let payload = BlockingTestPayload { started_sender, target_worker_receiver: Arc::new(Mutex::new(target_worker_receiver)), result_sender };

    // Trigger the blocking closure.
    MWFWrk::send_message(&EntityID::new_external("test-sender"), &sender_worker_id, &"start-blocking-work", &payload).await.unwrap();

    // Ensure the blocking closure has actually started.
    tokio::task::spawn_blocking(move || started_receiver.recv_timeout(Duration::from_secs(5))).await.unwrap().expect("Blocking closure did not start");

    // Abort the original worker and close the first MWF instance.
    MWFWrk::abort_worker(&sender_worker_id).unwrap();
    MWF::close().unwrap();

    // Initialize a new MWF instance.
    MWF::init_with_configuration(test_configuration()).unwrap();
    MWF::register_worker_type::<BlockingWorker>().unwrap();

    // Create a valid receiver in the new instance.
    let target_worker_id = MWFWrk::spawn_worker(&BlockingWorker::worker_type_id(), &parent_id).unwrap();

    MWFWrk::init_worker(&target_worker_id, &()).await.unwrap();
    MWFWrk::wait_worker_init(&target_worker_id, Some(5_000)).await.unwrap();

    // Release the old blocking closure.
    target_worker_sender.send(target_worker_id.clone()).unwrap();

    // Retrieve the result of its message attempt.
    let was_rejected = tokio::task::spawn_blocking(move || result_receiver.recv_timeout(Duration::from_secs(5))).await.unwrap().expect("Blocking closure did not return a result");

    // Clean up the second MWF instance.
    MWFWrk::close_worker(&target_worker_id, &()).await.unwrap();
    MWFWrk::wait_worker_close(&target_worker_id, Some(5_000)).await.unwrap();

    MWF::close().unwrap();

    assert!(was_rejected, "The old blocking closure successfully sent a message after MWF reinitialization");
}
