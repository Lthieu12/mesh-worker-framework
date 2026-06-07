//! Module definitions for MMF.
//!
//! This module defines the Module trait and the types used
//! to identify and manage modules within the framework.

use async_trait::async_trait;

use crate::message::FrameworkMessage;

/// Unique identifier of a module instance.
pub type ModuleID = String;

/// Identifier of a message sender.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SenderID {

    /// Message sent by a MMF module.
    Module(ModuleID),

    /// Message sent from outside MMF.
    External(String),
}

/// Current state of a module within the framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleState {

    /// Module has been created but not initialized.
    Created,
    /// Module initialization is in progress.
    Initializing,
    /// Module initialization has completed successfully.
    Initialized,
    /// Module Closing is in progress.
    Closing,
    /// Module has been closed and is not usable anymore.
    Closed,
}


/// Interface that must be implemented by all MMF modules.
#[async_trait]
pub trait Module {

    /// Called once when the module is initialized.
    async fn on_init(&mut self);

    /// Called when a message is received by the module.
    async fn on_message(&mut self, message: FrameworkMessage);

    /// Called once before the module is closed.
    async fn on_close(&mut self);
}