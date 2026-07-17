//! # Getting Started
//!
//! This example demonstrates the complete lifecycle of a MMF module.
//!
//! It shows how to:
//! - initialize the framework;
//! - register a module type;
//! - spawn and initialize a module;
//! - exchange messages;
//! - gracefully shut down the module and the framework.
//!
//! The `DemoModule` implementation is intentionally minimal so the
//! focus remains on the MMF API rather than application logic.

use bytes::Bytes;

mod modules;

use mesh_module_framework::*;
use modules::demo_module::{DemoMessages, DemoModule};

// Tokio provides the async runtime used by MMF.
#[tokio::main]
async fn main() -> Result<(), MMFError> {
    let main_entity = EntityID::new_external("MAIN");

    // Initialize MMF.
    MMF::init()?;

    // Register available module types.
    MMF::register_module_type::<DemoModule>()?;

    // Create the module.
    let demo_module_id = MMFMod::spawn_module("DemoModule", &main_entity)?;

    // Initialize it.
    MMFMod::init_module(&demo_module_id, Bytes::new()).await?;
    MMFMod::wait_module_init(&demo_module_id, None).await?;

    // Send a few messages.
    MMFMod::send_message(&main_entity, &demo_module_id, DemoMessages::QUICK_CALCULATION, Bytes::new()).await?;
    MMFMod::send_message(&main_entity, &demo_module_id, DemoMessages::LONG_CALCULATION, Bytes::new()).await?;

    // Close the module.
    MMFMod::close_module(&demo_module_id, Bytes::new()).await?;
    MMFMod::wait_module_close(&demo_module_id, None).await?;

    // Shutdown.
    MMF::close()?;

    Ok(())
}
