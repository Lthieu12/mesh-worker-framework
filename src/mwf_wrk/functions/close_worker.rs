use super::super::MWFWrk;
use crate::channel::WorkerCommand;
use crate::mwf::MWF;
use crate::mwf::MWF_WRK_DATA;
use crate::mwf::MWFError;
use crate::mwf::MWFPanic;
use crate::mwf_wrk::types::MessageData;
use crate::mwf_wrk::types::WorkerID;
use crate::mwf_wrk::types::WorkerState;
use crate::worker_task::current_worker_id;

impl MWFWrk {
    /// Request worker shutdown.
    ///
    /// The worker state is immediately changed to Closing.
    /// This function only queues the Close command and does not wait
    /// for on_close() completion.
    ///
    /// This function cannot be called by a worker to close itself.
    pub async fn close_worker<TCloseData>(worker_id: &WorkerID, close_data: &TCloseData) -> Result<(), MWFError>
    where
        TCloseData: Clone + Send + Sync + 'static,
    {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        // A worker cannot close itself.
        if current_worker_id().as_ref() == Some(worker_id) {
            return Err(MWFError::Worker_CannotManageSelf);
        }

        // Change worker state to Closing
        // and read MWF data to retrieve the worker channel
        // associated with the target worker.
        //
        // Only workers in the Initialized state can be closed.
        let normal_sender_channel = {
            let mut mwf_data_guard = MWF_WRK_DATA.write().expect(MWFPanic::MWFWrkData_WriteLockPoisoned.message());
            let mwf_data = mwf_data_guard.as_mut().ok_or(MWFError::Framework_NotInitialized)?;
            let worker_state = mwf_data.worker_registry.get_worker_state(worker_id)?;
            if worker_state != WorkerState::Initialized {
                return Err(MWFError::Worker_NotInitialized);
            }
            mwf_data.worker_registry.set_worker_state(worker_id, WorkerState::Closing)?;
            mwf_data.worker_registry.get_worker_normal_channel(worker_id)?
        };

        // Send the Close command to WorkerTask.
        normal_sender_channel.send(WorkerCommand::Close { close_data: MessageData::new(close_data) }).await.map_err(|_| MWFError::Worker_ChannelClosed)?;

        Ok(())
    }
}
