use super::super::MWFWrk;
use crate::mwf::MWFError;
use crate::mwf_wrk::types::WorkerID;

impl MWFWrk {
    /// Request closure of multiple workers.
    ///
    /// A Close command is queued for each worker sequentially, with its corresponding closing data.
    ///
    /// The state of each worker is immediately changed to Closing.
    /// This function does not wait for `on_close()` completion.
    ///
    /// Returns an error if the number of worker IDs does not match the number of closing data entries.
    /// Returns an error if one of the workers cannot be closed.
    /// If an error occurs, processing stops. Workers whose Close command was successfully queued
    /// before the error may already be in the Closing state. Subsequent workers are not processed.
    ///
    /// This function cannot be called by a worker to close itself.
    pub async fn close_worker_list<TCloseData>(worker_ids: &[WorkerID], close_data: &[TCloseData]) -> Result<(), MWFError>
    where
        TCloseData: Clone + Send + Sync + 'static,
    {
        if worker_ids.len() != close_data.len() {
            return Err(MWFError::Worker_DataCountMismatch);
        }
        for (worker_id, worker_close_data) in worker_ids.iter().zip(close_data.iter()) {
            MWFWrk::close_worker(worker_id, worker_close_data).await?;
        }

        Ok(())
    }
}
