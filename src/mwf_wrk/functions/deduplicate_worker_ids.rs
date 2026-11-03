use super::super::MWFWrk;
use crate::mwf_wrk::types::WorkerID;
use std::collections::HashSet;

impl MWFWrk {
    /// Removes duplicate WorkerIDs while preserving their original order.
    pub(crate) fn deduplicate_worker_ids(worker_ids: &[WorkerID]) -> Vec<WorkerID> {
        let mut seen = HashSet::with_capacity(worker_ids.len());
        let mut unique_worker_ids = Vec::with_capacity(worker_ids.len());

        for worker_id in worker_ids {
            if seen.insert(worker_id) {
                unique_worker_ids.push(worker_id.clone());
            }
        }

        unique_worker_ids
    }
}
