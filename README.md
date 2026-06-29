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

```text
MMF::init()?;

// Register module types once.
MMF::register_module_type::<CameraModule>()?;
MMF::register_module_type::<DisplayModule>()?;

// Create module instances.
let camera = MMFMod::spawn_module("CameraModule", &EntityID::External("MAIN".into()))?;

let display = MMFMod::spawn_module("DisplayModule", &EntityID::External("MAIN".into()))?;

// Initialize them.
MMFMod::init_module(camera, camera_config).await?;
MMFMod::init_module(display, display_config).await?;

// Send messages.
MMFMod::send_message(&EntityID::External("MAIN".into()), &camera, &"Start".into(), Bytes::new()).await?;

// Shutdown.
MMFMod::close_module(&camera, Bytes::new()).await?;
MMF::close()?;
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

This allows runtime debugging in both development and production environments.

---

### Module Type Registration

Before any module instance can be spawned,
its module type must be registered with MMF.

Each module implementation defines its own unique
`ModuleTypeID` through the Module trait.

Module types are typically registered once during
application startup:

```text
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

### Send

Fire-and-forget message.

---

### SendPriority

Fire-and-forget message processed before normal messages.

Priority messages are handled using a Last-In First-Out
(LIFO) policy.

---

### SendAndWait

The current processing flow is suspended until the target
module finishes processing the message or a timeout occurs.

The caller receives the processing result.

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

* Windows
* Linux

No platform-specific application code should be required.

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

## 13. Documentation

Additional design information is available in:

- architecture.md

