use super::super::MWFWrk;
use crate::mwf::MWF;
use crate::mwf::MWFError;
use crate::mwf_wrk::types::WorkerError;
use crate::wait_registry::complete_message_flow;
use crate::worker_task::current_message_inheritance_context;
use crate::worker_task::current_worker_id;

impl MWFWrk {
    /// Complete the MessageFlow associated with the currently processed message.
    ///
    /// The supplied result represents the outcome of the application-level processing
    /// and is forwarded to the caller waiting with WaitThisMessageFlow.
    ///
    /// MessageFlow completion is explicit. Returning an error from on_message() does not
    /// automatically complete the flow. The worker must call this function when its
    /// application-level processing is finished, including when that processing fails.
    ///
    /// If the caller has already timed out after the message was accepted, the MessageFlow
    /// remains active until this function is called or the WorkerTask terminates.
    ///
    /// Returns:
    /// - Worker_CurrentContextUnavailable if called outside an MWF worker task.
    /// - MessageFlow_NotActive if the current message has no active flow.
    pub fn message_flow_end(message_flow_result: Result<(), WorkerError>) -> Result<(), MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        // Identify the worker executing the current callback.
        let worker_id = current_worker_id().ok_or(MWFError::Worker_CurrentContextUnavailable)?;

        // Retrieve the message flow associated with the currently processed message.
        let message_flow_id = current_message_inheritance_context().message_flow_id.ok_or(MWFError::MessageFlow_NotActive)?;

        // Remove the flow and forward its result to the waiting caller.
        complete_message_flow(message_flow_id, &worker_id, message_flow_result)
    }
}
