# Mesh Module Framework (MMF)

## 1. Why MMF?

Mesh Module Framework (MMF) is a Rust Tokio/Futures-based runtime for building applications composed of long-running software modules that communicate through asynchronous messages.

Typical applications include:

- Industrial automation
- Scientific instrumentation
- Image processing pipelines
- Robotics
- Data acquisition systems

Unlike general-purpose actor frameworks, MMF focuses on:

- Deterministic execution
- Explicit module lifecycle management
- Type-safe module and message identifiers
- Module-to-module and external-to-module messaging
- Message prioritization
- Parent/child hierarchy management
- Message completion tracking
- Centralized logging and diagnostics
- Cross-platform support (Windows and Linux)

MMF intentionally remains application-agnostic. Business concepts such as workflows, measurements and calculations are left outside the framework.

```text
                          MMF
               Framework Entry Point
      (Initialization / Shutdown / Configuration)

                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
         ▼                   ▼                   ▼
 +----------------+   +----------------+   +----------------+
 |     MMFMod     |   |     MMFLog     |   |     MMFErr     |
 +----------------+   +----------------+   +----------------+
 | Messaging      |   | Log events     |   | Error events   |
 | Lifecycle      |   | Monitoring     |   | Monitoring     |
 | Registry       |   | Export         |   | Export         |
 +----------------+   +----------------+   +----------------+
```

Throughout this documentation, "Module" always refers to an MMF runtime module, not a text language module.

---

## 2. Quick Start

The complete, compiled version of this example is available in `examples/getting_started`.
It demonstrates the recommended way to initialize MMF, register a module type, spawn a module, exchange messages and gracefully shut down the framework.

```rust,ignore
use bytes::Bytes;

use mesh_module_framework::*;

// DemoModule and DemoMessages are defined in
// examples/getting_started/modules/demo_module.rs.
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
```

The following DemoModule implementation, available in `examples/getting_started/modules/demo_module.rs`, is used by the previous example and illustrates the minimum implementation required to create an MMF module.

```rust,ignore
use std::time::Duration;
use bytes::Bytes;
use async_trait::async_trait;
use mesh_module_framework::*;

/// Message identifiers handled by DemoModule.
pub struct DemoMessages;

/// Defines the message identifiers.
impl DemoMessages {
    pub const QUICK_CALCULATION: &'static str = "QUICK_CALCULATION";
    pub const LONG_CALCULATION: &'static str = "LONG_CALCULATION";
}

/// Private data associated with a DemoModule instance.
pub struct DemoModule {
    module_id: Option<ModuleID>,
    father_id: Option<EntityID>,
}

/// MMF module implementation for DemoModule.
#[async_trait]
impl Module for DemoModule {
    fn module_type_id() -> ModuleTypeID {
        "DemoModule"
    }

    fn new() -> Self {
        Self { module_id: None, father_id: None, }
    }

    async fn on_init(&mut self, module_id: ModuleID, father_id: EntityID, _lifecycle_data: Bytes) -> Result<(), ModuleError> {
        // Store identifiers for later use (for example to send messages or
        // spawn child modules from within the module).
        self.module_id = Some(module_id);
        self.father_id = Some(father_id);

        Ok(())
    }

    async fn on_message(&mut self, message: ModuleMessage) -> Result<(), ModuleError> {
        
        match message.message_id.as_str() {
            DemoMessages::QUICK_CALCULATION => {
                // Small blocking work performed directly by the module.
                std::thread::sleep(Duration::from_millis(10));
            }

            DemoMessages::LONG_CALCULATION => {
                // Long blocking work should be offloaded to a dedicated thread.
                MMFMod::spawn_blocking(|| {
                    std::thread::sleep(Duration::from_millis(1000));
                })
                .await?;
            }

            _ => {
                // Ignore unsupported messages.
            }
        }

        Ok(())
    }

    async fn on_close(&mut self, _lifecycle_data: Bytes) -> Result<(), ModuleError> {

        Ok(())
    }
}
```

---

## 3. Core Principles

### One Module = One Message at a Time

A module never processes multiple messages simultaneously.

This guarantees deterministic behavior and simplifies communication with hardware devices, external APIs and long-running calculations.

Parallelism is achieved by running multiple modules concurrently, not by processing multiple messages concurrently inside a single module.

---

### Framework-Agnostic Message Data

MMF transports opaque Message Data (`Bytes`).

The framework does not know anything about application-specific data structures.

Serialization and deserialization are always the responsibility of the application.

---

### Explicit Lifecycle

Every module implements:

* `on_init()`
* `on_message()`
* `on_close()`

These lifecycle methods are mandatory.

---

### Centralized Error Handling

Errors returned by module callbacks are automatically
captured by MMFErr.

Applications can monitor error events without requiring
individual modules to implement their own error reporting
mechanisms.

MMFErr can also be used by application code to record
custom errors, making it suitable as a centralized
error logging and monitoring system.

---

### Centralized Logging and Observability

MMF can log all message traffic, including:

* SenderID
* ReceiverID
* MessageID
* MessageInstanceID

This provides complete message traceability during development 
and production.

---

### Module Type Registration

Before any module instance can be spawned,
its module type must be registered with MMF.

Each module implementation defines its own unique
`ModuleTypeID` through the Module trait.

Module types are typically registered once during
application startup:

```rust,ignore
MMF::init()?;

MMF::register_module_type::<CameraModule>()?;
MMF::register_module_type::<FFTModule>()?;
MMF::register_module_type::<DisplayModule>()?;
```

Once registered, module instances can be spawned normally.

This registration step allows MMF to associate each
`ModuleTypeID` with its corresponding Rust module type.

Future MMF features, such as workflow-based module
instantiation, will rely on this information.

---

## 4. Communication Patterns

All messaging functions are asynchronous.

MMF uses asynchronous backpressure.
When a module queue reaches its configured capacity, message senders are suspended asynchronously until space becomes available. 
This prevents message loss while allowing the Tokio runtime to continue executing other modules.
Backpressure is transparent to the application and never blocks a Tokio worker thread.

### Send

Fire-and-forget message.

---

### SendPriority

Fire-and-forget message processed before normal messages.

Priority messages are handled using a Last-In, First-Out
(LIFO) policy.

---

### SendAndWait

The current processing flow is suspended until the target
module finishes processing the message or a timeout occurs.

The caller receives the processing result.

---

### Other Messaging Patterns

MMF also provides additional messaging patterns for more
specialized use cases:

* SendPriorityAndWait
* SendUnique

See the `MMFMod` API documentation for the complete semantics
and usage of these messaging functions.

---

## 5. Error Handling

MMF provides a centralized error collection system.

Errors generated by module callbacks are automatically
recorded by MMFErr.

Application code may also publish custom error events
through MMFErr::add_error().

Supported callback sources include:

* on_init()
* on_message()
* on_close()

Error events are stored in a circular buffer and can be
retrieved asynchronously by monitoring tools, loggers or
application code.

Each error event contains:

* ErrorInstanceID
* Timestamp
* SenderID
* Error message
* Error source chain

---

## 6. Logging and Observability

MMF provides a centralized logging and observability system.

Framework lifecycle events and message processing are automatically recorded by MMFLog.

Supported framework events include:

* Module spawn
* Module initialization
* Module shutdown
* Module abort
* Message reception

Log events are stored in a circular buffer and can be retrieved asynchronously by monitoring tools, loggers or application code.

Each log event contains:

* LogInstanceID
* Timestamp
* ModuleID
* Event
* SenderID (when applicable)
* MessageID (when applicable)
* Priority flag (when applicable)
* Wait flag (when applicable)
* Status
* Duration

---

## 7. Architecture Overview

### Runtime architecture

```text
                           MMF Runtime

                    ┌────────────────────┐
                    │        MMF         │
                    └─────────┬──────────┘
                              │
      ┌───────────────────────┼────────────────────────┐
      │                       │                        │
      ▼                       ▼                        ▼
┌────────────┐        ┌────────────┐          ┌────────────────┐
│   MMFMod   │        │   MMFLog   │          │     MMFErr     │
├────────────┤        ├────────────┤          ├────────────────┤
│ Registry   │        │ Log buffer │          │ Error buffer   │
│ Messaging  │        │ Waiters    │          │ Waiters        │
│ Lifecycle  │        │ Filters    │          │                │
└─────┬──────┘        └────────────┘          └────────────────┘
      │
      ▼
┌──────────────────────────────────────────────────────────────┐
│                         ModuleTask                           │
│                                                              │
│  • owns the Module instance                                  │
│  • executes lifecycle callbacks                              │
└──────────────────────────────┬───────────────────────────────┘
                               │ owns
                               ▼
                   ┌───────────────────────────┐
                   │        User Module        │
                   │   (implements Module)     │
                   ├───────────────────────────┤
                   │ on_init()                 │
                   │ on_message()              │
                   │ on_close()                │
                   └───────────────────────────┘
```

---

### Application architecture

```text
                   Application Startup

                       MMF::init()
                             │
                             ▼
           MMF::register_module_type::<CameraModule>()
           MMF::register_module_type::<FFTModule>()
           MMF::register_module_type::<DisplayModule>()
                             │
                             ▼
            ──────────── Runtime ────────────
                             │
                             ▼
                 CameraModule::spawn(...)
                 FFTModule::spawn(...)
                 DisplayModule::spawn(...)
                             │
                             ▼
                    ┌──────────────────┐
                    │     MMFMod       │
                    └────────┬─────────┘
                             │
                spawns one Tokio task per module
                             │
                             ▼
        ┌─────────────┬─────────────┬─────────────┐
        ▼             ▼             ▼
   +-----------+ +-----------+ +---------------+
   |ModuleTask | |ModuleTask | |  ModuleTask   |
   +-----┬-----+ +-----┬-----+ +-------┬-------+
         │             │               │
         ▼             ▼               ▼
   CameraModule    FFTModule     DisplayModule
```

---

### Architecture main rule

Each module owns its own mailbox and processes messages sequentially.

Parallelism is achieved by running multiple modules concurrently, not by processing multiple messages concurrently inside a single module.

---

## 8. Design Goals

* Easy integration
* Minimal boilerplate
* Cross-platform support
* Deterministic behavior
* Centralized logging
* Runtime observability
* Automated testing

---

## 9. Platform Support

MMF is designed to be fully cross-platform and currently supports:

* Windows
* Linux

No platform-specific application code should be required when developing modules with MMF.

To help guarantee cross-platform compatibility, the repository includes a verification script (scripts/verify.ps1) that automatically validates the framework on both Windows and Linux (WSL2). The script performs formatting, linting, testing, example compilation, and documentation generation on each supported platform.

---

## 10. Quality Requirements

MMF includes automated tests.

A build must fail if a test fails.

The long-term objective is to validate:

* Compilation
* Unit tests
* Integration tests
* Windows compatibility
* Linux compatibility

---

## 11. Project Status

The core runtime is implemented and continuously evolving.

Current work focuses on additional runtime features,
workflow support and production hardening.

---

## 12. Running Tests

MMF currently uses a global runtime instance.

Integration tests that call:

- MMF::init()
- MMF::close()

must not execute concurrently.

The project uses the `serial_test` crate and integration tests are annotated with:

```text
#[serial]
```

to ensure exclusive access to the MMF runtime during execution.

### Why are integration tests serialized?

MMF uses a global runtime instance stored in a static variable.

Running multiple integration tests concurrently could cause
Framework_AlreadyInitialized errors because several tests would
attempt to initialize the same MMF runtime simultaneously.


For this reason, integration tests using MMF are marked with
```text
#[serial]
```

---

## 13. Runtime overhead benchmark

MMF includes a runtime stress test that measures the framework overhead
without any application-specific processing.

Run it with:

```bash
cargo run --release --example runtime_overhead_benchmark
```

The number of modules can optionally be specified:

```bash
cargo run --release --example runtime_overhead_benchmark -- 100
cargo run --release --example runtime_overhead_benchmark -- 10000
```

The stress test measures:

- Module creation
- Module initialization
- Parallel `send_message_and_wait()`
- Module shutdown
- Overall throughput

Example output:

```text
=====================================================
            MMF runtime overhead benchmark  
=====================================================    
                                              
Modules : 1000                                             
                                                
Spawn                   :  3.775ms (3.77 µs/module)
Init                    : 662.700µs (0.66 µs/module)
SendAndWait (parallel)  :  1.307ms (1.31 µs/message)
Close                   :  3.248ms (3.25 µs/module)

Total                   :  8.992ms
Throughput              : 765345 msg/s
```

These measurements represent the runtime overhead of MMF using
`SimulationModule` with `SimulationBehavior::DoNothing`.
Actual application performance depends on the work performed by each module.

---

## 14. Documentation

Additional design information is available in:

- architecture.md

