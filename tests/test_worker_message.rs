//! Integration tests for MWF message handling.
//!
//! Test scenarios:
//!
//! - `test_message_priority_order`:
//!   - Start processing a normal message.
//!   - Queue additional normal messages while the first message is being processed.
//!   - Wait until the second normal message starts processing.
//!   - Send priority messages while another normal message is pending.
//!   - Verify that normal messages are processed using FIFO ordering.
//!   - Verify that priority messages are processed using LIFO ordering.
//!   - Verify that priority messages are processed before normal messages already pending in the internal queue.
//!
//! - `test_send_message_and_wait`:
//!   - Create two workers.
//!   - Configure the first worker to send a message to the second worker
//!     and wait for its completion.
//!   - Trigger the message chain.
//!   - Verify that both workers process the expected messages.
//!   - Verify that the waiting mode and sender identity are correctly reported.
//!
//! - `test_send_unique_message`:
//!   - Keep a worker busy processing a message.
//!   - Send several unique messages with the same MessageID.
//!   - Verify that pending unique messages replace previous ones.
//!   - Verify that only the last unique message is processed after the current message.
//!
//! - `test_send_priority_message_racing_with_worker_close`:
//!   - Fill the worker's priority channel while it is processing a message.
//!   - Start another priority message send and verify that it waits for channel capacity.
//!   - Request worker shutdown while the priority send is waiting.
//!   - Verify that the pending send returns Worker_NotInitialized after the worker enters Closing.
//!   - Verify that send_priority_message_and_wait() times out while waiting
//!     for channel capacity for both message completion modes.
//!   - Verify that messages accepted before shutdown are processed before the Close command.
//!   - Verify that the rejected message is never processed.
//!
//! - `test_multiple_workers_send_and_wait_to_same_receiver`:
//!   - Create three workers.
//!   - Configure two workers to send a message to the third worker
//!     and wait for its completion.
//!   - Trigger both senders concurrently.
//!   - Verify that both senders can wait for the same receiver simultaneously.
//!   - Verify that the receiver processes the messages sequentially.
//!   - Verify that the second sender waits for the first message to finish
//!     before its own message is processed.
//!   - Verify that both senders receive successful completion results.
use serial_test::serial;
use std::future::Future;
use std::time::Duration;

use mwf::*;

use dev_support::dev_helper::*;
use dev_support::simulation_worker::{SimulationBehavior, SimulationWorker, SimulationWorkerConfig};

const MESSAGE_NORMAL_1: &str = "NORMAL_1";
const MESSAGE_NORMAL_2: &str = "NORMAL_2";
const MESSAGE_NORMAL_3: &str = "NORMAL_3";
const MESSAGE_PRIORITY_1: &str = "PRIORITY_1";
const MESSAGE_PRIORITY_2: &str = "PRIORITY_2";
const MESSAGE_TIMEOUT: &str = "TIMEOUT";
const MESSAGE_CHAIN: &str = "CHAIN";

// MWF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_message_priority_order() {
    MWF::init().unwrap();

    MWFLog::set_log_mode(LogMode::LogAll, None).unwrap();

    MWF::register_worker_type::<SimulationWorker>().unwrap();

    let worker_a_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &EntityID::new_external("MAIN")).unwrap();

    let configuration = SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::Message_Wait { message_delay_ms: 500 }, wait_children_close: false, wait_children_close_timeout_ms: None };

    MWFWrk::init_worker(&worker_a_id, &configuration).await.unwrap();

    MWFWrk::wait_worker_init(&worker_a_id, Some(1000)).await.unwrap();

    // Retrieve the latest log ID before sending messages.
    let mut last_log_instance_id = MWFLog::wait_log(None).await.unwrap().expect("Expected log events").last_log_instance_id;

    // Start processing the first normal message.
    MWFWrk::send_message(&EntityID::new_external("MAIN"), &worker_a_id, &MESSAGE_NORMAL_1, &()).await.unwrap();

    // Wait until the first normal message callback has started.
    loop {
        let log_list = tokio::time::timeout(Duration::from_secs(5), MWFLog::wait_log(last_log_instance_id)).await.expect("NORMAL_1 did not start").unwrap().expect("Expected log events");

        last_log_instance_id = log_list.last_log_instance_id;

        if log_list.log_event_list.iter().any(|log_event| {
            log_event.worker_id == worker_a_id
                && log_event.log_status == LogStatus::Start
                && matches!(
                    &log_event.log_action,
                    LogAction::WorkerMessage { message_id, .. }
                        if message_id.as_str() == MESSAGE_NORMAL_1
                )
        }) {
            break;
        }
    }

    // Queue two normal messages while NORMAL_1 is being processed.
    MWFWrk::send_message(&EntityID::new_external("MAIN"), &worker_a_id, &MESSAGE_NORMAL_2, &()).await.unwrap();

    MWFWrk::send_message(&EntityID::new_external("MAIN"), &worker_a_id, &MESSAGE_NORMAL_3, &()).await.unwrap();

    // Wait until NORMAL_2 starts processing.
    //
    // At this point, NORMAL_3 has already been collected into PendingCommands.
    // Priority messages sent now must bypass this pending normal command.
    loop {
        let log_list = tokio::time::timeout(Duration::from_secs(5), MWFLog::wait_log(last_log_instance_id)).await.expect("NORMAL_2 did not start").unwrap().expect("Expected log events");

        last_log_instance_id = log_list.last_log_instance_id;

        if log_list.log_event_list.iter().any(|log_event| {
            log_event.worker_id == worker_a_id
                && log_event.log_status == LogStatus::Start
                && matches!(
                    &log_event.log_action,
                    LogAction::WorkerMessage { message_id, .. }
                        if message_id.as_str() == MESSAGE_NORMAL_2
                )
        }) {
            break;
        }
    }

    // Send priority messages while NORMAL_2 is being processed.
    MWFWrk::send_priority_message(&EntityID::new_external("MAIN"), &worker_a_id, &MESSAGE_PRIORITY_1, &()).await.unwrap();

    MWFWrk::send_priority_message(&EntityID::new_external("MAIN"), &worker_a_id, &MESSAGE_PRIORITY_2, &()).await.unwrap();

    // NORMAL_1 has already completed before NORMAL_2 starts.
    let mut completed_message_count = 1;

    // Wait until all five messages have been processed.
    while completed_message_count < 5 {
        let log_list = tokio::time::timeout(Duration::from_secs(5), MWFLog::wait_log(last_log_instance_id)).await.expect("Message processing did not complete").unwrap().expect("Expected log events");

        last_log_instance_id = log_list.last_log_instance_id;

        completed_message_count += log_list.log_event_list.iter().filter(|log_event| log_event.worker_id == worker_a_id && log_event.log_status == LogStatus::Ok && matches!(&log_event.log_action, LogAction::WorkerMessage { .. })).count();
    }

    let log_list = MWFLog::wait_log(None).await.unwrap().expect("Expected log events");

    let messages = filter_messages(&log_list, &worker_a_id);

    print_log_list().await;

    assert_eq!(messages.len(), 10);

    // NORMAL_1
    assert_eq!(messages[0].message_id.as_str(), MESSAGE_NORMAL_1);
    assert_eq!(messages[0].log_status, LogStatus::Start);
    assert!(!messages[0].priority);
    assert!(!messages[0].wait_completion);

    assert_eq!(messages[1].message_id.as_str(), MESSAGE_NORMAL_1);
    assert_eq!(messages[1].log_status, LogStatus::Ok);

    // NORMAL_2
    assert_eq!(messages[2].message_id.as_str(), MESSAGE_NORMAL_2);
    assert_eq!(messages[2].log_status, LogStatus::Start);
    assert!(!messages[2].priority);
    assert!(!messages[2].wait_completion);

    assert_eq!(messages[3].message_id.as_str(), MESSAGE_NORMAL_2);
    assert_eq!(messages[3].log_status, LogStatus::Ok);

    // PRIORITY_2
    assert_eq!(messages[4].message_id.as_str(), MESSAGE_PRIORITY_2);
    assert_eq!(messages[4].log_status, LogStatus::Start);
    assert!(messages[4].priority);
    assert!(!messages[4].wait_completion);

    assert_eq!(messages[5].message_id.as_str(), MESSAGE_PRIORITY_2);
    assert_eq!(messages[5].log_status, LogStatus::Ok);

    // PRIORITY_1
    assert_eq!(messages[6].message_id.as_str(), MESSAGE_PRIORITY_1);
    assert_eq!(messages[6].log_status, LogStatus::Start);
    assert!(messages[6].priority);
    assert!(!messages[6].wait_completion);

    assert_eq!(messages[7].message_id.as_str(), MESSAGE_PRIORITY_1);
    assert_eq!(messages[7].log_status, LogStatus::Ok);

    // NORMAL_3
    assert_eq!(messages[8].message_id.as_str(), MESSAGE_NORMAL_3);
    assert_eq!(messages[8].log_status, LogStatus::Start);
    assert!(!messages[8].priority);
    assert!(!messages[8].wait_completion);

    assert_eq!(messages[9].message_id.as_str(), MESSAGE_NORMAL_3);
    assert_eq!(messages[9].log_status, LogStatus::Ok);

    MWFWrk::close_worker(&worker_a_id, &()).await.unwrap();

    MWFWrk::wait_children_close(&EntityID::new_external("MAIN"), Some(1000)).await.unwrap();

    MWF::close().unwrap();
}

// MWF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_send_message_and_wait() {
    MWF::init().unwrap();

    MWFLog::set_log_mode(LogMode::LogAll, None).unwrap();

    MWF::register_worker_type::<SimulationWorker>().unwrap();

    //
    // Spawn worker B.
    //
    let worker_b_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &EntityID::new_external("MAIN")).unwrap();

    let configuration = SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::Message_Wait { message_delay_ms: 100 }, wait_children_close: false, wait_children_close_timeout_ms: None };

    MWFWrk::init_worker(&worker_b_id, &configuration).await.unwrap();

    MWFWrk::wait_worker_init(&worker_b_id, Some(1000)).await.unwrap();

    //
    // Spawn worker A.
    //
    let worker_a_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &EntityID::new_external("MAIN")).unwrap();

    let configuration = SimulationWorkerConfig {
        init_delay_ms: 0,
        close_delay_ms: 0,
        simulation_behavior: SimulationBehavior::Message_SendAndWait { target_worker_id: worker_b_id.clone(), message_id: MESSAGE_CHAIN.into(), timeout_ms: Some(1000), message_delay_ms: 0 },
        wait_children_close: false,
        wait_children_close_timeout_ms: None,
    };

    MWFWrk::init_worker(&worker_a_id, &configuration).await.unwrap();

    MWFWrk::wait_worker_init(&worker_a_id, Some(1000)).await.unwrap();

    //
    // Trigger the chain.
    //
    MWFWrk::send_message(&EntityID::new_external("MAIN"), &worker_a_id, &MESSAGE_CHAIN, &()).await.unwrap();

    //
    // Wait until processing completes.
    //
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;

    let log_list = MWFLog::wait_log(None).await.unwrap().expect("Expected log events");

    let messages_a = filter_messages(&log_list, &worker_a_id);
    let messages_b = filter_messages(&log_list, &worker_b_id);

    print_log_list().await;

    //
    // worker A.
    //
    assert_eq!(messages_a.len(), 2);

    assert_eq!(messages_a[0].message_id.as_str(), MESSAGE_CHAIN);
    assert_eq!(messages_a[0].log_status, LogStatus::Start);
    assert!(!messages_a[0].priority);
    assert!(!messages_a[0].wait_completion);

    assert_eq!(messages_a[1].message_id.as_str(), MESSAGE_CHAIN);
    assert_eq!(messages_a[1].log_status, LogStatus::Ok);

    //
    // worker B.
    //
    assert_eq!(messages_b.len(), 2);

    assert_eq!(messages_b[0].message_id.as_str(), MESSAGE_CHAIN);
    assert_eq!(messages_b[0].log_status, LogStatus::Start);
    assert!(!messages_b[0].priority);
    assert!(messages_b[0].wait_completion);

    assert_eq!(messages_b[0].sender_id, &EntityID::Worker(worker_a_id.clone()));

    assert_eq!(messages_b[1].message_id.as_str(), MESSAGE_CHAIN);
    assert_eq!(messages_b[1].log_status, LogStatus::Ok);

    MWFWrk::close_worker(&worker_a_id, &()).await.unwrap();
    MWFWrk::close_worker(&worker_b_id, &()).await.unwrap();

    MWFWrk::wait_children_close(&EntityID::new_external("MAIN"), Some(1000)).await.unwrap();

    MWF::close().unwrap();
}

// MWF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_send_unique_message() {
    // Initialize MWF.
    MWF::init().unwrap();

    MWFLog::set_log_mode(LogMode::LogAll, None).unwrap();

    MWF::register_worker_type::<SimulationWorker>().unwrap();

    // Spawn and initialize the simulation worker.
    let worker_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &EntityID::new_external("MAIN")).unwrap();

    let configuration = SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::Message_Wait { message_delay_ms: 100 }, wait_children_close: false, wait_children_close_timeout_ms: None };

    MWFWrk::init_worker(&worker_id, &configuration).await.unwrap();
    MWFWrk::wait_worker_init(&worker_id, Some(1000)).await.unwrap();

    // Send a blocking message to keep the worker busy.
    MWFWrk::send_message(&EntityID::new_external("MAIN"), &worker_id, &MESSAGE_CHAIN, &()).await.unwrap();

    // Give the worker enough time to start processing
    // the blocking message.
    MWFWrk::sleep(Duration::from_millis(20)).await;

    // Send several equivalent unique messages.
    for _ in 0..5 {
        MWFWrk::send_unique_message(&EntityID::new_external("MAIN"), &worker_id, &MESSAGE_CHAIN, &()).await.unwrap();
    }

    // Wait until all messages have been processed.
    MWFWrk::sleep(Duration::from_millis(300)).await;

    // Verify that only the first and the last unique
    // messages have been processed.
    let log_list = MWFLog::wait_log(None).await.unwrap().expect("Expected log events");
    print_log_list().await;

    let messages = filter_messages(&log_list, &worker_id);

    assert_eq!(messages.len(), 4);

    // Start of the blocking message.
    assert_eq!(messages[0].message_id.as_str(), &MESSAGE_CHAIN.to_string());
    assert_eq!(messages[0].log_status, LogStatus::Start);

    // End of the blocking message.
    assert_eq!(messages[1].message_id.as_str(), &MESSAGE_CHAIN.to_string());
    assert_eq!(messages[1].log_status, LogStatus::Ok);

    // Start of the last unique message.
    assert_eq!(messages[2].message_id.as_str(), &MESSAGE_CHAIN.to_string());
    assert_eq!(messages[2].log_status, LogStatus::Start);

    // End of the last unique message.
    assert_eq!(messages[3].message_id.as_str(), &MESSAGE_CHAIN.to_string());
    assert_eq!(messages[3].log_status, LogStatus::Ok);

    // The unique message must correspond to the fifth unique
    // message that was sent.
    assert_eq!(*messages[2].message_instance_id, *messages[0].message_instance_id + 5);

    assert_eq!(*messages[3].message_instance_id, *messages[2].message_instance_id);

    MWFWrk::close_worker(&worker_id, &()).await.unwrap();
    MWFWrk::wait_worker_close(&worker_id, Some(1000)).await.unwrap();

    MWF::close().unwrap();
}

// MWF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_send_priority_message_racing_with_worker_close() {
    const MESSAGE_BLOCKING: &str = "BLOCKING";
    const MESSAGE_QUEUED_PRIORITY: &str = "QUEUED_PRIORITY";
    const MESSAGE_PENDING: &str = "PENDING";

    // Use small channels to control the test scenario.
    let configuration = MWFConfiguration { worker_normal_channel_capacity: 1, worker_priority_channel_capacity: 1, log_file_path: None, ..Default::default() };

    MWF::init_with_configuration(configuration).unwrap();

    MWFLog::set_log_mode(LogMode::LogAll, None).unwrap();
    MWF::register_worker_type::<SimulationWorker>().unwrap();

    let worker_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &EntityID::new_external("MAIN")).unwrap();

    // Keep the worker busy long enough to fill its priority channel.
    let configuration = SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::Message_Wait { message_delay_ms: 500 }, wait_children_close: false, wait_children_close_timeout_ms: None };

    MWFWrk::init_worker(&worker_id, &configuration).await.unwrap();
    MWFWrk::wait_worker_init(&worker_id, Some(1000)).await.unwrap();

    // Retrieve the latest log ID before sending the first message.
    let mut last_log_instance_id = MWFLog::wait_log(None).await.unwrap().expect("Expected log events").last_log_instance_id;

    // Start processing the first message.
    MWFWrk::send_message(&EntityID::new_external("MAIN"), &worker_id, &MESSAGE_BLOCKING, &()).await.unwrap();

    // Wait until the first message callback has actually started.
    loop {
        let log_list = MWFLog::wait_log(last_log_instance_id).await.unwrap().expect("Expected log events");
        last_log_instance_id = log_list.last_log_instance_id;

        let blocking_message_started = log_list.log_event_list.iter().any(|log_event| {
            log_event.worker_id == worker_id
                && log_event.log_status == LogStatus::Start
                && matches!(
                    &log_event.log_action,
                    LogAction::WorkerMessage { message_id, .. }
                        if message_id.as_str() == MESSAGE_BLOCKING
                )
        });

        if blocking_message_started {
            break;
        }
    }

    // Fill the priority channel while the worker is processing the first message.
    MWFWrk::send_priority_message(&EntityID::new_external("MAIN"), &worker_id, &MESSAGE_QUEUED_PRIORITY, &()).await.unwrap();

    assert_eq!(MWFWrk::get_worker_status(&worker_id).unwrap().priority_channel_size, 1);

    // Verify that both wait modes apply the timeout while waiting for channel capacity.
    for wait_mode in [MessageWaitMode::WaitThisMessage, MessageWaitMode::WaitThisMessageFlow] {
        let wait_result = tokio::time::timeout(Duration::from_millis(250), MWFWrk::send_priority_message_and_wait(&EntityID::new_external("MAIN"), &worker_id, &MESSAGE_TIMEOUT, wait_mode, &(), Some(50))).await;

        assert!(matches!(wait_result, Ok(Err(MWFError::Operation_Timeout))), "Expected Operation_Timeout for {wait_mode:?}, got: {wait_result:?}");
    }

    // Create a priority send that must wait for an available channel slot.
    let (pending_sender, pending_receiver) = tokio::sync::oneshot::channel();

    let sender_id = EntityID::new_external("MAIN");
    let receiver_id = worker_id.clone();
    let message_id = MESSAGE_PENDING.to_string();

    let send_task = tokio::spawn(async move {
        let message_data = ();

        let send_future = MWFWrk::send_priority_message(&sender_id, &receiver_id, &message_id, &message_data);
        tokio::pin!(send_future);

        let mut pending_sender = Some(pending_sender);

        // Signal the test when the send future first becomes pending.
        // At this point, the initial worker-state validation has succeeded
        // and reserve() is waiting for channel capacity.
        std::future::poll_fn(|context| {
            let result = send_future.as_mut().poll(context);

            if result.is_pending() {
                if let Some(pending_sender) = pending_sender.take() {
                    let _ = pending_sender.send(());
                }
            }

            result
        })
        .await
    });

    tokio::time::timeout(Duration::from_secs(2), pending_receiver).await.expect("Priority send did not reach channel reservation").expect("Priority send task terminated before waiting for channel capacity");

    // Start closing the worker while the priority send is waiting for capacity.
    MWFWrk::close_worker(&worker_id, &()).await.unwrap();

    // The waiting send must detect the Closing state after obtaining its reservation.
    let send_result = tokio::time::timeout(Duration::from_secs(3), send_task).await.expect("Priority send did not complete after the channel became available").expect("Priority send task panicked");

    // Wait until the worker has completely terminated.
    MWFWrk::wait_worker_close(&worker_id, Some(3000)).await.unwrap();

    let log_list = MWFLog::wait_log(None).await.unwrap().expect("Expected log events");

    // Collect the messages whose callbacks actually started.
    let processed_message_ids = log_list
        .log_event_list
        .iter()
        .filter_map(|log_event| {
            if log_event.worker_id != worker_id || log_event.log_status != LogStatus::Start {
                return None;
            }

            match &log_event.log_action {
                LogAction::WorkerMessage { message_id, .. } => Some(message_id.as_str().to_owned()),
                _ => None,
            }
        })
        .collect::<Vec<_>>();

    MWF::close().unwrap();

    // The message waiting for capacity must be rejected.
    assert!(matches!(send_result, Err(MWFError::Worker_NotInitialized)), "Expected Worker_NotInitialized, got: {send_result:?}");

    // Both messages accepted before closing must have been processed.
    // The message waiting for capacity must never reach the callback.
    assert_eq!(processed_message_ids, vec![MESSAGE_BLOCKING.to_string(), MESSAGE_QUEUED_PRIORITY.to_string()]);
}

// MWF uses a global runtime instance.
// Integration tests must run serially.
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_multiple_workers_send_and_wait_to_same_receiver() {
    MWF::init().unwrap();

    MWFLog::set_log_mode(LogMode::LogAll, None).unwrap();

    MWF::register_worker_type::<SimulationWorker>().unwrap();

    // Spawn and initialize receiver C.
    let worker_c_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &EntityID::new_external("MAIN")).unwrap();

    let configuration_c = SimulationWorkerConfig { init_delay_ms: 0, close_delay_ms: 0, simulation_behavior: SimulationBehavior::Message_Wait { message_delay_ms: 250 }, wait_children_close: false, wait_children_close_timeout_ms: None };

    MWFWrk::init_worker(&worker_c_id, &configuration_c).await.unwrap();
    MWFWrk::wait_worker_init(&worker_c_id, Some(1000)).await.unwrap();

    // Spawn and initialize senders A and B.
    let worker_a_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &EntityID::new_external("MAIN")).unwrap();

    let worker_b_id = MWFWrk::spawn_worker(&SimulationWorker::worker_type_id(), &EntityID::new_external("MAIN")).unwrap();

    let configuration_a = SimulationWorkerConfig {
        init_delay_ms: 0,
        close_delay_ms: 0,
        simulation_behavior: SimulationBehavior::Message_SendAndWait { target_worker_id: worker_c_id.clone(), message_id: MESSAGE_CHAIN.into(), timeout_ms: Some(2000), message_delay_ms: 0 },
        wait_children_close: false,
        wait_children_close_timeout_ms: None,
    };

    let configuration_b = SimulationWorkerConfig {
        init_delay_ms: 0,
        close_delay_ms: 0,
        simulation_behavior: SimulationBehavior::Message_SendAndWait { target_worker_id: worker_c_id.clone(), message_id: MESSAGE_CHAIN.into(), timeout_ms: Some(2000), message_delay_ms: 0 },
        wait_children_close: false,
        wait_children_close_timeout_ms: None,
    };

    MWFWrk::init_worker(&worker_a_id, &configuration_a).await.unwrap();
    MWFWrk::init_worker(&worker_b_id, &configuration_b).await.unwrap();

    MWFWrk::wait_worker_init(&worker_a_id, Some(1000)).await.unwrap();
    MWFWrk::wait_worker_init(&worker_b_id, Some(1000)).await.unwrap();

    let message_id_a = MESSAGE_CHAIN.to_string();
    let message_id_b = MESSAGE_CHAIN.to_string();

    let sender_id_a = EntityID::new_external("MAIN");
    let sender_id_b = EntityID::new_external("MAIN");

    // Trigger both senders concurrently and wait for their completion.
    let (result_a, result_b) = tokio::join!(MWFWrk::send_message_and_wait(&sender_id_a, &worker_a_id, &message_id_a, MessageWaitMode::WaitThisMessage, &(), Some(2000),), MWFWrk::send_message_and_wait(&sender_id_b, &worker_b_id, &message_id_b, MessageWaitMode::WaitThisMessage, &(), Some(2000),),);

    assert!(result_a.is_ok(), "Worker A failed: {result_a:?}");
    assert!(result_b.is_ok(), "Worker B failed: {result_b:?}");

    // Display framework logs after both message flows have completed.
    print_log_list().await;

    let log_list = MWFLog::wait_log(None).await.unwrap().expect("Expected log events");

    let messages_a = filter_messages(&log_list, &worker_a_id);
    let messages_b = filter_messages(&log_list, &worker_b_id);
    let messages_c = filter_messages(&log_list, &worker_c_id);

    // Each sender must have received its trigger and completed its send-and-wait.
    assert_eq!(messages_a.len(), 2);
    assert_eq!(messages_b.len(), 2);

    assert_eq!(messages_a[0].log_status, LogStatus::Start);
    assert_eq!(messages_a[1].log_status, LogStatus::Ok);

    assert_eq!(messages_b[0].log_status, LogStatus::Start);
    assert_eq!(messages_b[1].log_status, LogStatus::Ok);

    // Receiver C must have processed both messages.
    assert_eq!(messages_c.len(), 4);
    assert_eq!(messages_c[0].log_status, LogStatus::Start);
    assert_eq!(messages_c[1].log_status, LogStatus::Ok);
    assert_eq!(messages_c[2].log_status, LogStatus::Start);
    assert_eq!(messages_c[3].log_status, LogStatus::Ok);

    // The second sender must include the first message's processing time in its wait duration.
    let duration_a_ms = messages_a[1].log_event.duration_ms.unwrap();
    let duration_b_ms = messages_b[1].log_event.duration_ms.unwrap();

    let max_duration_ms = duration_a_ms.max(duration_b_ms);

    assert!(max_duration_ms >= 400, "Expected the second sender to wait for the first message to finish, but durations were A={duration_a_ms}ms and B={duration_b_ms}ms");

    MWFWrk::close_worker(&worker_a_id, &()).await.unwrap();
    MWFWrk::close_worker(&worker_b_id, &()).await.unwrap();
    MWFWrk::close_worker(&worker_c_id, &()).await.unwrap();

    MWFWrk::wait_children_close(&EntityID::new_external("MAIN"), Some(1000)).await.unwrap();

    MWF::close().unwrap();
}
