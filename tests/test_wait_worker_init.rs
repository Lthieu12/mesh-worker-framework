//! Tests for worker initialization wait notifications.
//!
//! This file verifies that `MWFWrk::wait_worker_init()` does not remain blocked
//! when a worker is removed before reaching the Initialized state.
//!
//! Test scenarios:
//!
//! - `wait_worker_init_returns_not_found_after_abort()`:
//!   1. Initialize MWF and register a worker type.
//!   2. Spawn a worker without initializing it.
//!   3. Start waiting for its initialization and verify that the wait is pending.
//!   4. Abort the worker.
//!   5. Verify that the wait terminates with `Err(MWFError::Worker_NotFound)`.
//!   6. Close MWF.
//!
//! - `wait_worker_init_returns_not_found_after_init_panic()`:
//!   1. Initialize MWF and register a worker that panics during `on_init()`.
//!   2. Spawn the worker.
//!   3. Start waiting for its initialization and verify that the wait is pending.
//!   4. Request worker initialization, triggering the panic.
//!   5. Verify that the wait terminates with `Err(MWFError::Worker_NotFound)`.
//!   6. Close MWF.
//!
//! Both tests synchronize on the wait future entering the Pending state before
//! triggering the worker abort or initialization.
//! A timeout detects any wait operation that remains blocked unexpectedly.
use serial_test::serial;
use std::future::Future;
use std::sync::Mutex;
use std::task::Poll;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use mwf::{EntityID, MWF, MWFConfiguration, MWFError, MWFWrk, MessageData, Worker, WorkerError, WorkerMessage, WorkerTypeID};

// Serialize tests sharing the global MWF state.
static TEST_MUTEX: Mutex<()> = Mutex::new(());

/// Starts a future and signals when it has actually returned Pending.
fn spawn_wait_until_pending<F>(future: F) -> (JoinHandle<F::Output>, oneshot::Receiver<()>)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let (pending_sender, pending_receiver) = oneshot::channel();

    let waiter_handle = tokio::spawn(async move {
        let mut future = Box::pin(future);
        let mut pending_sender = Some(pending_sender);

        std::future::poll_fn(move |context| match future.as_mut().poll(context) {
            Poll::Pending => {
                if let Some(sender) = pending_sender.take() {
                    let _ = sender.send(());
                }

                Poll::Pending
            }

            Poll::Ready(result) => Poll::Ready(result),
        })
        .await
    });

    (waiter_handle, pending_receiver)
}

/// Initializes MWF without creating a log file.
fn init_test_mwf() {
    let mut configuration = MWFConfiguration::default();
    configuration.log_file_path = None;

    MWF::init_with_configuration(configuration).expect("Failed to initialize MWF");
}

/// Worker used to test abort during initialization.
struct AbortTestWorker;

#[async_trait]
impl Worker for AbortTestWorker {
    fn worker_type_id() -> WorkerTypeID {
        WorkerTypeID::new("WorkerWaitInitAbortTest")
    }

    fn new() -> Self {
        Self
    }

    async fn on_init(&mut self, _init_data: MessageData) -> Result<(), WorkerError> {
        Ok(())
    }

    async fn on_message(&mut self, _message: WorkerMessage) -> Result<(), WorkerError> {
        Ok(())
    }

    async fn on_close(&mut self, _close_data: MessageData) -> Result<(), WorkerError> {
        Ok(())
    }
}

/// Worker that intentionally panics during initialization.
struct PanicTestWorker;

#[async_trait]
impl Worker for PanicTestWorker {
    fn worker_type_id() -> WorkerTypeID {
        WorkerTypeID::new("WorkerWaitInitPanicTest")
    }

    fn new() -> Self {
        Self
    }

    async fn on_init(&mut self, _init_data: MessageData) -> Result<(), WorkerError> {
        panic!("Intentional panic during on_init");
    }

    async fn on_message(&mut self, _message: WorkerMessage) -> Result<(), WorkerError> {
        Ok(())
    }

    async fn on_close(&mut self, _close_data: MessageData) -> Result<(), WorkerError> {
        Ok(())
    }
}

/// Verifies that wait_worker_init() is awakened when its worker is aborted.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn wait_worker_init_returns_not_found_after_abort() {
    let _test_guard = TEST_MUTEX.lock().expect("Test mutex poisoned");

    init_test_mwf();
    MWF::register_worker_type::<AbortTestWorker>().unwrap();

    let parent_id = EntityID::new_external("test-parent");
    let worker_id = MWFWrk::spawn_worker(&AbortTestWorker::worker_type_id(), &parent_id).unwrap();

    // Start waiting while the worker is still in the Created state.
    let (waiter_handle, pending_receiver) = spawn_wait_until_pending({
        let worker_id = worker_id.clone();
        async move { MWFWrk::wait_worker_init(&worker_id, None).await }
    });

    pending_receiver.await.expect("wait_worker_init() did not enter the pending state");

    // Abort the worker while initialization is still pending.
    MWFWrk::abort_worker(&worker_id).unwrap();

    // Verify that the waiting task is awakened and returns Worker_NotFound.
    let result = tokio::time::timeout(Duration::from_secs(5), waiter_handle).await.expect("wait_worker_init() remained blocked after worker abort").expect("wait_worker_init() task panicked");

    assert!(matches!(result, Err(MWFError::Worker_NotFound)), "Expected Err(Worker_NotFound) after abort, got {result:?}");

    MWF::close().expect("Failed to close MWF");
}

/// Verifies that wait_worker_init() is awakened when its worker panics during on_init().
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn wait_worker_init_returns_not_found_after_init_panic() {
    let _test_guard = TEST_MUTEX.lock().expect("Test mutex poisoned");

    init_test_mwf();
    MWF::register_worker_type::<PanicTestWorker>().unwrap();

    let parent_id = EntityID::new_external("test-parent");
    let worker_id = MWFWrk::spawn_worker(&PanicTestWorker::worker_type_id(), &parent_id).unwrap();

    // Start waiting while the worker is still in the Created state.
    let (waiter_handle, pending_receiver) = spawn_wait_until_pending({
        let worker_id = worker_id.clone();
        async move { MWFWrk::wait_worker_init(&worker_id, None).await }
    });

    pending_receiver.await.expect("wait_worker_init() did not enter the pending state");

    // Trigger initialization. The worker intentionally panics in on_init().
    MWFWrk::init_worker(&worker_id, &()).await.unwrap();

    // Verify that the waiting task is awakened and returns Worker_NotFound.
    let result = tokio::time::timeout(Duration::from_secs(5), waiter_handle).await.expect("wait_worker_init() remained blocked after initialization panic").expect("wait_worker_init() task panicked");

    assert!(matches!(result, Err(MWFError::Worker_NotFound)), "Expected Err(Worker_NotFound) after initialization panic, got {result:?}");

    MWF::close().expect("Failed to close MWF");
}
