use crate::MWFConfiguration;
use crate::worker_type_registry::WorkerTypeRegistry;

/// Internal MWF runtime data.
#[derive(Debug)]
pub(crate) struct MWFData {
    /// Registered worker types.
    pub(crate) worker_type_registry: WorkerTypeRegistry,

    /// Configuration used to initialize MWF.
    pub(crate) configuration: MWFConfiguration,
}

impl MWFData {
    /// Creates the internal MWF runtime data.
    pub(crate) fn new(configuration: MWFConfiguration) -> Self {
        Self { worker_type_registry: WorkerTypeRegistry::new(), configuration }
    }
}
