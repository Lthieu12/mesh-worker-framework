//! Development helper functions.
//!
//! This file provides utility functions used by
//! examples, integration tests and runtime stress tests.
//!
//! These helpers are intended for development only and
//! are not part of the MWF runtime itself.

use crate::MWFErr;
use crate::MWFLog;
use crate::MWFWrk;

use crate::message::{MessageID, MessageInstanceID};
use crate::mwf_log::{LogAction, LogEvent, LogList, LogStatus};
use crate::worker::{EntityID, WorkerID};

#[allow(dead_code)]
pub fn print_worker_list() {
    let worker_list = MWFWrk::get_worker_info_list().unwrap();
    println!("{worker_list}");
}

#[allow(dead_code)]
pub async fn print_log_list() {
    let log_list = MWFLog::wait_log(None).await.unwrap();
    println!("{log_list}");
}

#[allow(dead_code)]
pub async fn print_error_list() {
    let error_list = MWFErr::wait_error(None).await.unwrap();
    println!("{error_list}");
}

#[allow(dead_code)]
pub struct ReceiveMessageLog<'a> {
    pub log_event: &'a LogEvent,
    pub message_instance_id: &'a MessageInstanceID,
    pub sender_id: &'a EntityID,
    pub message_id: &'a MessageID,
    pub priority: bool,
    pub wait_completion: bool,
    pub log_status: LogStatus,
}

/// Returns all ReceiveMessage log events received by a worker.
///
/// The returned events preserve the original chronological order.
#[allow(dead_code)]
pub fn filter_messages<'a>(log_list: &'a LogList, worker_id: &WorkerID) -> Vec<ReceiveMessageLog<'a>> {
    let mut receive_messages = Vec::new();

    for log_event in &log_list.log_event_list {
        if log_event.worker_id != *worker_id {
            continue;
        }

        if let LogAction::WorkerMessage { message_instance_id, sender_id, message_id, priority, wait_completion } = &log_event.log_action {
            receive_messages.push(ReceiveMessageLog { log_event, message_instance_id, sender_id, message_id, priority: *priority, wait_completion: *wait_completion, log_status: log_event.log_status });
        }
    }

    receive_messages
}
