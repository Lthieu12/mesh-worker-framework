use super::super::MWFWrk;
use crate::channel::WorkerCommand;
use crate::framework_error::MWFError;
use crate::framework_error::MWFPanic;
use crate::message::MessageData;
use crate::mwf::MWF;
use crate::mwf::MWF_WRK_DATA;
use crate::worker::WorkerID;
use crate::worker::WorkerState;
use crate::worker_task::current_worker_id;

impl MWFWrk {
    /// Request worker initialization.
    ///
    /// The worker state is immediately changed to Initializing.
    /// This function only queues the Init command and does not wait
    /// for on_init() completion.
    ///
    /// This function cannot be called by a worker to initialize itself.
    pub async fn init_worker<TInitData>(worker_id: &WorkerID, init_data: &TInitData) -> Result<(), MWFError>
    where
        TInitData: Clone + Send + Sync + 'static,
    {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        // A worker cannot initialize itself.
        if current_worker_id().as_ref() == Some(worker_id) {
            return Err(MWFError::Worker_CannotManageSelf);
        }

        // Change worker state to Initializing
        // and read MWF data to retrieve the worker channel
        // associated with the target worker.
        //
        // Only workers in the Created state can be initialized.
        let normal_sender_channel = {
            let mut mwf_data_guard = MWF_WRK_DATA.write().expect(MWFPanic::MWFWrkData_WriteLockPoisoned.message());
            let mwf_data = mwf_data_guard.as_mut().ok_or(MWFError::Framework_NotInitialized)?;
            let worker_state = mwf_data.worker_registry.get_worker_state(worker_id)?;
            if worker_state != WorkerState::Created {
                return Err(MWFError::Worker_AlreadyInitialized);
            }
            mwf_data.worker_registry.set_worker_state(worker_id, WorkerState::Initializing)?;

            mwf_data.worker_registry.get_worker_normal_channel(worker_id)?
        };

        // Send the Init command to WorkerTask.
        normal_sender_channel.send(WorkerCommand::Init { init_data: MessageData::new(init_data) }).await.map_err(|_| MWFError::Worker_ChannelClosed)?;

        Ok(())
    }
}
