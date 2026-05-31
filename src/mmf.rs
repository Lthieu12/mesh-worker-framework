use std::sync::{LazyLock, RwLock};
use crate::error::FrameworkError;
use crate::registry::Registry;
use crate::logging::LoggingData;

static MMF_DATA: LazyLock<RwLock<Option<MMFData>>> = LazyLock::new(|| RwLock::new(None));

struct MMFData {
    registry: Registry,
    logging_data: LoggingData,
}

impl MMFData {
    fn new() -> Self {
        Self {
            registry: Registry::new(),
            logging_data: LoggingData::new(),
        }
    }
}

pub struct MMF;

impl MMF {

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

    pub(crate) fn is_initialized() -> bool {
        MMF_DATA.read().unwrap().is_some()
    }
}


