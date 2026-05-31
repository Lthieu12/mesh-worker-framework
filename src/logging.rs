use crate::module::ModuleID;
//use crate::error::FrameworkError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogMode {
    NoLog,
    LogSelected,
    LogAll,
}

pub(crate) struct LoggingData {
    pub log_mode: LogMode,
    pub logged_module_list: Vec<ModuleID>,
}

impl LoggingData {
    pub(crate) fn new() -> Self {
        Self {
            log_mode: LogMode::NoLog,
            logged_module_list: Vec::new(),
        }
    }
}