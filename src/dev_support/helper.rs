//! Development helper functions.
//!
//! This module provides utility functions used by
//! examples, integration tests and runtime stress tests.
//!
//! These helpers are intended for development only and
//! are not part of the MMF runtime itself.
use crate::MMFLog;
use crate::MMFErr;
use crate::MMFMod;

use crate::module::{ModuleID, EntityID};
use crate::message::{MessageID};
use crate::mmf_log::{LogAction, LogStatus, LogEvent, LogList};

#[allow(dead_code)]
pub fn print_module_list() {

    let module_list = MMFMod::get_module_info_list().unwrap();
    println!("{module_list}");
}


#[allow(dead_code)]
pub async fn print_log_list() {

    let log_list = MMFLog::wait_log(None).await.unwrap();
    println!("{log_list}");
}


#[allow(dead_code)]
pub async fn print_error_list() {

    let error_list = MMFErr::wait_error(None).await.unwrap();
    println!("{error_list}");
}


#[allow(dead_code)]
pub struct ReceiveMessageLog<'a> {

    pub log_event: &'a LogEvent,
    pub sender_id: &'a EntityID,
    pub message_id: &'a MessageID,
    pub priority: bool,
    pub wait_completion: bool,
    pub log_status: LogStatus,
}

/// Returns all ReceiveMessage log events received by a module.
///
/// The returned events preserve the original chronological order.
#[allow(dead_code)]
pub fn filter_messages<'a>(log_list: &'a LogList, module_id: &ModuleID) -> Vec<ReceiveMessageLog<'a>> {

    let mut receive_messages = Vec::new();

    for log_event in &log_list.log_event_list {

        if log_event.module_id != *module_id {
            continue;
        }

        if let LogAction::ModuleMessage {
            sender_id,
            message_id,
            priority,
            wait_completion,
        } = &log_event.log_action {

            receive_messages.push(
                ReceiveMessageLog {
                    log_event,
                    sender_id,
                    message_id,
                    priority: *priority,
                    wait_completion: *wait_completion,
                    log_status: log_event.log_status,
                }
            );
        }
    }

    receive_messages
}