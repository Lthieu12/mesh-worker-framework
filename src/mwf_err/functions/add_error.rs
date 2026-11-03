use super::super::MWFErr;
use crate::MWF;
use crate::mwf::MWFError;
use crate::mwf_wrk::types::EntityID;

impl MWFErr {
    /// Record a new application error event.
    ///
    /// This function allows application code to report
    /// custom errors through the MWFErr infrastructure.
    ///
    /// MWF internal errors are recorded through a dedicated
    /// internal API that automatically enriches the error
    /// context.
    ///
    /// Error events are stored in a circular buffer and
    /// notify all MWFErr waiters.
    pub fn add_error(error_source: &EntityID, error: &anyhow::Error) -> Result<(), MWFError> {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        // Validate the entity identifier.
        if !error_source.is_valid() {
            return Err(MWFError::Entity_Invalid { entity_id: error_source.clone() });
        }
        MWFErr::add_error_internal(error_source, None, error)
    }
}
