use super::super::MWFWrk;
use crate::mwf::MWFError;
use crate::mwf_wrk::types::WorkerID;

impl MWFWrk {
    /// Request workers initialization.
    ///
    /// Each worker is initialized sequentially by queuing an Init command
    /// with its corresponding initializing data.
    ///
    /// The worker state is immediately changed to Initializing.
    /// This function does not wait for `on_init()` completion.
    ///
    /// Returns an error if the number of worker IDs does not match the number of initializing data entries.
    /// Returns an error if one of the workers cannot be initialized.
    /// Workers processed before the error may already have received their Init command.
    ///
    /// This function cannot be called by a worker to initialize itself.
    pub async fn init_worker_list<TInitData>(worker_ids: &[WorkerID], init_data: &[TInitData]) -> Result<(), MWFError>
    where
        TInitData: Clone + Send + Sync + 'static,
    {
        if worker_ids.len() != init_data.len() {
            return Err(MWFError::Worker_DataCountMismatch);
        }
        for (worker_id, worker_init_data) in worker_ids.iter().zip(init_data.iter()) {
            MWFWrk::init_worker(worker_id, worker_init_data).await?;
        }

        Ok(())
    }
}
