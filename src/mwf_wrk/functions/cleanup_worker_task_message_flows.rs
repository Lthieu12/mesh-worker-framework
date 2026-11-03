use crate::mwf::MWF_MESSAGE_WAIT_DATA;
use crate::mwf::MWFError;
use crate::mwf_wrk::types::WorkerID;

/// Removes all active message flows owned by a worker task and notifies their waiters.
///
/// The registry lock is released before notifying waiters.
pub(crate) fn cleanup_worker_task_message_flows(worker_id: &WorkerID) -> Result<(), MWFError> {
    // Remove all flows owned by this worker while holding the registry lock.
    let flow_states = {
        let mut message_wait_data_guard = MWF_MESSAGE_WAIT_DATA.write().map_err(|_| MWFError::MessageWait_RegistryInconsistent)?;

        // Nothing to clean up if the framework data has already been destroyed.
        let Some(message_wait_data) = message_wait_data_guard.as_mut() else {
            return Ok(());
        };

        let message_flow_ids = message_wait_data.message_flow_registry.iter().filter_map(|(message_flow_id, flow_state)| (&flow_state.worker_id == worker_id).then_some(*message_flow_id)).collect::<Vec<_>>();

        message_flow_ids.into_iter().filter_map(|message_flow_id| message_wait_data.message_flow_registry.remove(&message_flow_id)).collect::<Vec<_>>()
    };

    // Notify waiters only after releasing the registry lock.
    for flow_state in flow_states {
        let _ = flow_state.reply_sender.send(Err(MWFError::MessageFlow_WorkerTaskTerminated { worker_id: worker_id.clone() }));
    }

    Ok(())
}
