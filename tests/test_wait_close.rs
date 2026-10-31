//! Tests for MWF wait functions during framework shutdown.
//!
//! This file verifies that application tasks waiting for log or error events
//! are properly awakened when MWF is closed.
//!
//! Test scenarios:
//! - wait_log_returns_none_when_mwf_is_closed():
//!   1. Initialize MWF without file logging.
//!   2. Start a task waiting for new log events.
//!   3. Verify that the task has entered the pending state.
//!   4. Close MWF while the task is waiting.
//!   5. Verify that the waiting task terminates successfully with Ok(None).
//!
//! - wait_error_returns_none_when_mwf_is_closed():
//!   1. Initialize MWF without file logging.
//!   2. Start a task waiting for new error events.
//!   3. Verify that the task has entered the pending state.
//!   4. Close MWF while the task is waiting.
//!   5. Verify that the waiting task terminates successfully with Ok(None).
//!
//! Both tests use synchronization to ensure that the wait operation is
//! actually pending before closing MWF, avoiding timing-dependent delays.
use serial_test::serial;
use std::future::Future;
use std::sync::Mutex;
use std::task::Poll;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use mwf::{MWF, MWFConfiguration, MWFErr, MWFLog};

// Serialize tests sharing the global MWF state.
static TEST_MUTEX: Mutex<()> = Mutex::new(());

/// Starts a future and signals when it has actually returned Pending.
///
/// This ensures the tested wait function is registered as a waiter
/// before the framework is closed.
fn spawn_wait_until_pending<F>(future: F) -> (JoinHandle<F::Output>, oneshot::Receiver<()>)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let (pending_sender, pending_receiver) = oneshot::channel();

    let waiter_handle = tokio::spawn(async move {
        let mut future = Box::pin(future);
        let mut pending_sender = Some(pending_sender);

        std::future::poll_fn(move |context| {
            match future.as_mut().poll(context) {
                Poll::Pending => {
                    // Notify the test only once, after the future is pending.
                    if let Some(sender) = pending_sender.take() {
                        let _ = sender.send(());
                    }

                    Poll::Pending
                }

                Poll::Ready(result) => Poll::Ready(result),
            }
        })
        .await
    });

    (waiter_handle, pending_receiver)
}

/// Verifies that wait_log() returns Ok(None) when MWF is closed
/// while the application is waiting for new log events.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn wait_log_returns_none_when_mwf_is_closed() {
    let _test_guard = TEST_MUTEX.lock().expect("Test mutex poisoned");

    // Initialize MWF without creating a log file.
    let mut configuration = MWFConfiguration::default();
    configuration.log_file_path = None;

    MWF::init_with_configuration(configuration).expect("Failed to initialize MWF");

    // Start waiting for log events and ensure the future is pending.
    let (waiter_handle, pending_receiver) = spawn_wait_until_pending(MWFLog::wait_log(None));

    pending_receiver.await.expect("wait_log() did not enter the pending state");

    // Close MWF while the log waiter is active.
    MWF::close().expect("Failed to close MWF");

    // Verify that the waiter terminates normally with Ok(None).
    let result = waiter_handle.await.expect("wait_log() task panicked");

    assert!(matches!(result, Ok(None)), "Expected Ok(None) after MWF close, got {result:?}");
}

/// Verifies that wait_error() returns Ok(None) when MWF is closed
/// while the application is waiting for new error events.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn wait_error_returns_none_when_mwf_is_closed() {
    let _test_guard = TEST_MUTEX.lock().expect("Test mutex poisoned");

    // Initialize MWF without creating a log file.
    let mut configuration = MWFConfiguration::default();
    configuration.log_file_path = None;

    MWF::init_with_configuration(configuration).expect("Failed to initialize MWF");

    // Start waiting for error events and ensure the future is pending.
    let (waiter_handle, pending_receiver) = spawn_wait_until_pending(MWFErr::wait_error(None));

    pending_receiver.await.expect("wait_error() did not enter the pending state");

    // Close MWF while the error waiter is active.
    MWF::close().expect("Failed to close MWF");

    // Verify that the waiter terminates normally with Ok(None).
    let result = waiter_handle.await.expect("wait_error() task panicked");

    assert!(matches!(result, Ok(None)), "Expected Ok(None) after MWF close, got {result:?}");
}
