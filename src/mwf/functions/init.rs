use super::super::MWF;
use super::super::MWFConfiguration;
use crate::mwf::MWFError;

impl MWF {
    /// Initialize MWF using the default configuration.
    ///
    /// Each successful initialization after a previous shutdown receives a new
    /// instance ID, used to ensure WorkerID uniqueness within the current process.
    pub fn init() -> Result<(), MWFError> {
        Self::init_with_configuration(MWFConfiguration::default())
    }
}
