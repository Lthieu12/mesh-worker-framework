//! Public MMF API.
//!
//! This module exposes the framework entry points used to
//! initialize, shutdown and manage modules.

use std::sync::{LazyLock, RwLock};

use crate::error::FrameworkError;
use crate::logging::LoggingData;
use crate::module::{Module, ModuleID};
use crate::registry::Registry;

/// Global MMF runtime storage.
static MMF_DATA: LazyLock<RwLock<Option<MMFData>>> = LazyLock::new(|| RwLock::new(None));

/// Internal MMF runtime data.
struct MMFData {

    /// Registered modules and hierarchy information.
    registry: Registry,

    /// Logging configuration.
    logging_data: LoggingData,
}

impl MMFData {

    /// Create a new MMF runtime instance.
    fn new() -> Self {
        Self {
            registry: Registry::new(),
            logging_data: LoggingData::new(),
        }
    }
}

/// Public MMF entry point.
pub struct MMF;

impl MMF {

    /// Initialize the MMF framework.
    pub fn init() -> Result<(), FrameworkError> {
        {
            let mut mmf_data = MMF_DATA.write().unwrap();

            if mmf_data.is_some() {
                return Err(FrameworkError::FrameworkAlreadyInitialized);
            }

            *mmf_data = Some(MMFData::new());
        }

        Ok(())
    }

    /// Shutdown the MMF framework and release all resources.
    pub fn close() -> Result<(), FrameworkError> {
        {
            let mut mmf_data = MMF_DATA.write().unwrap();

            if mmf_data.is_none() {
                return Err(FrameworkError::FrameworkNotInitialized);
            }

            *mmf_data = None;
        }

        Ok(())
    }

    /// Check whether the framework is initialized.
    pub(crate) fn is_initialized() -> bool {
        MMF_DATA.read().unwrap().is_some()
    }

    /// Spawn a new module and wait for its initialization.
    pub fn spawn_module<T>(father_module_id: Option<ModuleID>, module: T) -> Result<ModuleID, FrameworkError> where T: Module + Send + 'static {
        
        let mut mmf_data_guard = MMF_DATA.write().unwrap();

        let mmf_data = mmf_data_guard.as_mut().ok_or(FrameworkError::FrameworkNotInitialized)?;

        let module_id = mmf_data.registry.generate_module_id::<T>();

        Ok(module_id)
    }
}


#[cfg(test)]
mod tests {

    use crate::error::FrameworkError;
    use crate::mmf::MMF;

    #[test]
    fn test_mmf_init_close() {
        assert!(!MMF::is_initialized());

        assert!(MMF::init().is_ok());

        assert!(MMF::is_initialized());

        assert!(MMF::close().is_ok());

        assert!(!MMF::is_initialized());
    }

    #[test]
    fn test_mmf_double_init() {
        assert!(MMF::init().is_ok());

        assert!(
            matches!(
                MMF::init(),
                Err(FrameworkError::FrameworkAlreadyInitialized)
            )
        );

        assert!(MMF::close().is_ok());
    }

    #[test]
    fn test_mmf_double_close() {
        assert!(
            matches!(
                MMF::close(),
                Err(FrameworkError::FrameworkNotInitialized)
            )
        );
    }
}