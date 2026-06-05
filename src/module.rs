//! Module definitions for MMF.
//!
//! This module defines the Module trait and the types used
//! to identify and manage modules within the framework.

use crate::message::FrameworkMessage;

/// Unique identifier of a module instance.
pub type ModuleID = String;

/// Current state of a module within the framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModuleState {

    /// Module has been created but not initialized.
    Created,

    /// Module initialization has completed successfully.
    Initialized,
}


/// Interface that must be implemented by all MMF modules.
#[allow(async_fn_in_trait)]
pub trait Module {

    /// Called once when the module is initialized.
    async fn on_init(&mut self);

    /// Called when a message is received by the module.
    async fn on_message(&mut self, message: FrameworkMessage);

    /// Called once before the module is closed.
    async fn on_close(&mut self);
}