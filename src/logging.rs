use crate::module::ModuleID;
use crate::error::FrameworkError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogMode {
    NoLog,
    LogSelected,
    LogAll,
}

pub struct LoggingData {
    pub log_mode: LogMode,
    pub logged_module_list: Vec<ModuleID>,
}