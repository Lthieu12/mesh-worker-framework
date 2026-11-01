//! Tests preventing workers from managing or waiting for their own lifecycle.
//!
//! Test steps:
//! 1. Initialize MWF and register a dedicated test worker.
//! 2. During on_init(), attempt to initialize and wait for the current worker.
//! 3. Verify that both operations return Worker_CannotManageSelf or
//!    Worker_CannotWaitForSelf as appropriate.
//! 4. During on_message(), attempt to close and wait for the current worker.
//! 5. Verify that both operations return the expected errors.
//! 6. Close the worker from an external task.
//! 7. Verify that the worker closes normally and shut down MWF.
use async_trait::async_trait;
use serial_test::serial;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;
use tokio::time::{Duration, timeout};

use mwf::*;

// MWF uses a global runtime instance.
// Integration tests must run serially.

type SelfLifecycleResults = (Result<(), MWFError>, Result<(), MWFError>);

type ReportSender = Arc<Mutex<Option<oneshot::Sender<SelfLifecycleResults>>>>;

#[derive(Clone)]
struct SelfLifecycleWorkerConfig {
    init_report_sender: ReportSender,
    message_report_sender: ReportSender,
}

struct SelfLifecycleWorker {
    message_report_sender: Option<ReportSender>,
}

fn send_report(report_sender: &ReportSender, results: SelfLifecycleResults) {
    let sender = report_sender.lock().unwrap().take();

    if let Some(sender) = sender {
        let _ = sender.send(results);
    }
}

#[async_trait]
impl Worker for SelfLifecycleWorker {
    fn worker_type_id() -> WorkerTypeID {
        "SelfLifecycleWorker".into()
    }

    fn new() -> Self {
        Self { message_report_sender: None }
    }

    async fn on_init(&mut self, init_data: MessageData) -> Result<(), WorkerError> {
        let configuration = init_data.into::<SelfLifecycleWorkerConfig>()?;

        let worker_id = MWFWrk::get_current_worker_id()?;

        // A worker must not initialize or wait for its own initialization.
        let init_self_result = MWFWrk::init_worker(&worker_id, &()).await;
        let wait_self_result = MWFWrk::wait_worker_init(&worker_id, Some(100)).await;

        send_report(&configuration.init_report_sender, (init_self_result, wait_self_result));

        self.message_report_sender = Some(configuration.message_report_sender);

        Ok(())
    }

    async fn on_message(&mut self, _message: WorkerMessage) -> Result<(), WorkerError> {
        let worker_id = MWFWrk::get_current_worker_id()?;

        // A worker must not close or wait for its own closure.
        let close_self_result = MWFWrk::close_worker(&worker_id, &()).await;
        let wait_self_result = MWFWrk::wait_worker_close(&worker_id, Some(100)).await;

        if let Some(report_sender) = &self.message_report_sender {
            send_report(report_sender, (close_self_result, wait_self_result));
        }

        Ok(())
    }

    async fn on_close(&mut self, _close_data: MessageData) -> Result<(), WorkerError> {
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_worker_cannot_manage_or_wait_for_itself() {
    MWF::init().unwrap();
    MWF::register_worker_type::<SelfLifecycleWorker>().unwrap();

    let (init_sender, init_receiver) = oneshot::channel();
    let (message_sender, message_receiver) = oneshot::channel();

    let configuration = SelfLifecycleWorkerConfig { init_report_sender: Arc::new(Mutex::new(Some(init_sender))), message_report_sender: Arc::new(Mutex::new(Some(message_sender))) };

    let parent_id = EntityID::new_external("TEST");

    let worker_id = MWFWrk::spawn_worker(&SelfLifecycleWorker::worker_type_id(), &parent_id).unwrap();

    // Initialize the worker and retrieve the results of its self-checks.
    MWFWrk::init_worker(&worker_id, &configuration).await.unwrap();

    let init_results = timeout(Duration::from_secs(5), init_receiver).await.expect("Initialization self-check remained blocked").expect("Initialization self-check report was not received");

    MWFWrk::wait_worker_init(&worker_id, Some(5_000)).await.unwrap();

    // Send a message to execute the shutdown self-checks.
    MWFWrk::send_message_and_wait(&parent_id, &worker_id, &MessageID::new("SELF_LIFECYCLE_CHECK"), MessageWaitMode::WaitThisMessage, &(), Some(5_000)).await.unwrap();

    let message_results = timeout(Duration::from_secs(5), message_receiver).await.expect("Shutdown self-check remained blocked").expect("Shutdown self-check report was not received");

    // Close the worker externally.
    MWFWrk::close_worker(&worker_id, &()).await.unwrap();
    MWFWrk::wait_worker_close(&worker_id, Some(5_000)).await.unwrap();

    MWF::close().unwrap();

    // Verify lifecycle management protections.
    assert!(matches!(init_results.0, Err(MWFError::Worker_CannotManageSelf)));
    assert!(matches!(init_results.1, Err(MWFError::Worker_CannotWaitForSelf)));

    assert!(matches!(message_results.0, Err(MWFError::Worker_CannotManageSelf)));
    assert!(matches!(message_results.1, Err(MWFError::Worker_CannotWaitForSelf)));
}
