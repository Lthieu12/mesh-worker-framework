use crate::error::FrameworkError;
use crate::registry::Registry;
use crate::logging::LoggingData;

pub struct FrameworkData {
    pub registry: Registry,
    pub logging_data: LoggingData,
}