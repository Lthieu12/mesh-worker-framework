use super::super::MWF;
use super::super::MWF_DATA;
use crate::framework_error::MWFPanic;

impl MWF {
    /// Check whether the framework is initialized.
    pub fn is_initialized() -> bool {
        MWF_DATA.read().expect(MWFPanic::MWFData_ReadLockPoisoned.message()).is_some()
    }
}
