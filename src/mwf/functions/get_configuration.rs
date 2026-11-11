use super::super::MWF;
use super::super::MWF_DATA;
use super::super::MWFConfiguration;
use crate::mwf::MWFError;
use crate::mwf::MWFPanic;

impl MWF {
    /// Returns a copy of the configuration used to initialize MWF.
    ///
    /// Returns Framework_NotInitialized if MWF has not been initialized.
    pub fn get_configuration() -> Result<MWFConfiguration, MWFError> {
        let mwf_data_guard = MWF_DATA.read().expect(MWFPanic::MWFData_ReadLockPoisoned.message());

        let mwf_data = mwf_data_guard.as_ref().ok_or(MWFError::Framework_NotInitialized)?;

        Ok(mwf_data.configuration.clone())
    }
}
