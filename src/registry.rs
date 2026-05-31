use std::collections::HashMap;
use crate::module::{ModuleID, ModuleState};
//use crate::error::FrameworkError;

#[derive(Debug, Clone)]
pub(crate) struct RegistryEntry {
    father_module_id: Option<ModuleID>,
    module_state: ModuleState,
    //TODO: pub module_channel: ModuleChannel;
}

pub(crate) struct Registry {
    module_map: HashMap<ModuleID, RegistryEntry>,
    child_module_map: HashMap<ModuleID, Vec<ModuleID>>,
}

impl Registry {
    pub(crate) fn new() -> Self {
        Self {
            module_map: HashMap::new(),
            child_module_map: HashMap::new(),
        }
    }
}