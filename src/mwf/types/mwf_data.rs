use crate::worker_type_registry::WorkerTypeRegistry;

/// Internal MWF runtime data.
#[derive(Debug)]
pub(crate) struct MWFData {
    /// Registered worker types.
    pub(crate) worker_type_registry: WorkerTypeRegistry,
}

impl MWFData {
    pub(crate) fn new() -> Self {
        Self { worker_type_registry: WorkerTypeRegistry::new() }
    }
}
