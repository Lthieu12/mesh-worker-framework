use std::collections::HashMap;
use crate::module::{ModuleID, ModuleState};
use crate::error::FrameworkError;

#[derive(Debug, Clone)]
pub struct RegistryEntry {
    pub father_module_id: Option<ModuleID>,
    pub module_state: ModuleState,
    //TODO: pub module_channel: ModuleChannel;
}

pub struct Registry {
    pub module_map: HashMap<ModuleID, RegistryEntry>,
    pub child_module_map: HashMap<ModuleID, Vec<ModuleID>>,
}