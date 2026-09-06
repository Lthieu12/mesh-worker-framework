# Mesh Worker Framework (MWF)

## 1. Why MWF?

Mesh Worker Framework (MWF) is a Rust Tokio/Futures-based runtime for building applications composed of long-running software workers that communicate through asynchronous messages.

Typical applications include:

- Industrial automation
- Scientific instrumentation
- Image processing pipelines
- Robotics
- Data acquisition systems

Unlike general-purpose actor frameworks, MWF focuses on:

- Deterministic execution
- Explicit worker lifecycle management
- Type-safe worker and message identifiers
- Worker-to-worker and external-to-worker messaging
- Message prioritization
- Parent/child hierarchy management
- Message completion tracking
- Centralized logging and diagnostics
- Cross-platform support (Windows and Linux)

MWF intentionally remains application-agnostic. Business concepts such as workflows, measurements and calculations are left outside the framework.

```text
                          MWF
               Framework Entry Point
      (Initialization / Shutdown / Configuration)

                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
         ▼                   ▼                   ▼
 +----------------+   +----------------+   +----------------+
 |     MWFWrk     |   |     MWFLog     |   |     MWFErr     |
 +----------------+   +----------------+   +----------------+
 | Workers        |   | Log events     |   | Error events   |
 | Lifecycle      |   | Monitoring     |   | Monitoring     |
 | Messaging      |   | Export         |   | Export         |
 +----------------+   +----------------+   +----------------+
```

---

## 2. Quick Start

The complete, compiled version of this example is available in `examples/getting_started`.
It demonstrates the recommended way to initialize MWF, register a worker type, spawn a worker, exchange messages and gracefully shut down the framework.

```rust,ignore
use mwf::*;

// DemoWorker and DemoMessages are defined in
// examples/getting_started/workers/demo_worker.rs.
mod workers;
use workers::demo_worker::{DemoMessages, DemoWorker};

// Tokio provides the async runtime used by MWF.
#[tokio::main]
async fn main() -> Result<(), MWFError> {
    let main_entity = EntityID::new_external("MAIN");

    // Initialize MWF.
    MWF::init()?;

    // Register available worker types.
    MWF::register_worker_type::<DemoWorker>()?;

    // Create the worker.
    let demo_worker_id = MWFWrk::spawn_worker(&DemoWorker::worker_type_id(), &main_entity)?;

    // Initialize it.
    MWFWrk::init_worker(&demo_worker_id, &()).await?;
    MWFWrk::wait_worker_init(&demo_worker_id, None).await?;

    // Send a few messages.
    MWFWrk::send_message(&main_entity, &demo_worker_id, &DemoMessages::QUICK_CALCULATION, &()).await?;
    MWFWrk::send_message(&main_entity, &demo_worker_id, &DemoMessages::LONG_CALCULATION, &()).await?;

    // Close the worker.
    MWFWrk::close_worker(&demo_worker_id, &()).await?;
    MWFWrk::wait_worker_close(&demo_worker_id, None).await?;

    // Shutdown.
    MWF::close()?;

    Ok(())
}
```

The following DemoWorker implementation, available in `examples/getting_started/workers/demo_worker.rs`, is used by the previous example and illustrates the minimum implementation required to create an MWF worker.

```rust,ignore
use async_trait::async_trait;
use mwf::*;
use std::time::Duration;

/// Message identifiers handled by DemoWorker.
pub struct DemoMessages;

/// Defines the message identifiers.
impl DemoMessages {
    pub const QUICK_CALCULATION: &'static str = "QUICK_CALCULATION";
    pub const LONG_CALCULATION: &'static str = "LONG_CALCULATION";
}

/// Private data associated with a DemoWorker instance.
pub struct DemoWorker {
    worker_id: Option<WorkerID>,
    parent_id: Option<EntityID>,
}

/// MWF worker implementation for DemoWorker.
#[async_trait]
impl Worker for DemoWorker {
    fn worker_type_id() -> WorkerTypeID {
        "DemoWorker".into()
    }

    fn new() -> Self {
        Self { worker_id: None, parent_id: None }
    }

    async fn on_init(&mut self, worker_id: WorkerID, parent_id: EntityID, _init_data: MessageData) -> Result<(), WorkerError> {
        // Store identifiers for later use (for example to send messages or
        // spawn child workers from within the worker).
        self.worker_id = Some(worker_id);
        self.parent_id = Some(parent_id);

        Ok(())
    }

    async fn on_message(&mut self, message: WorkerMessage) -> Result<(), WorkerError> {
        match message.message_id.as_str() {
            DemoMessages::QUICK_CALCULATION => {
                // Small blocking work performed directly by the worker.
                std::thread::sleep(Duration::from_millis(10));
            }

            DemoMessages::LONG_CALCULATION => {
                // Long blocking work should be offloaded to a dedicated thread.
                MWFWrk::spawn_blocking(|| {
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

    async fn on_close(&mut self, _close_data: MessageData) -> Result<(), WorkerError> {
        Ok(())
    }
}
```

---

## 3. Core Principles

### One Worker = One Message at a Time

A worker never processes multiple messages simultaneously.

This guarantees deterministic behavior and simplifies communication with hardware devices, external APIs and long-running calculations.

Parallelism is achieved by running multiple workers concurrently, not by processing multiple messages concurrently inside a single worker.

---

### Framework-Agnostic Typed Message Data

MWF transports opaque MessageData values.

MessageData can contain any Rust type implementing `Clone + Send + Sync + 'static`.

MWF never interprets the contained value and simply transports it between workers or lifecycle callbacks.

**`MessageData` stores a clone of the value provided by the application.** The original value remains owned by the caller while MWF owns an independent copy used during message transport.

Large shared objects should typically be transported using `Arc<T>` to avoid unnecessary copies.

Example:

```rust,ignore
struct HistogramParameters {
    bins: usize,
}

MWFWrk::send_message(
    &sender,
    &worker,
    &HistogramMessages::Compute,
    &HistogramParameters {
        bins: 256,
    },
)
.await?;
```

Inside the worker, retrieve the original type with:

```rust,ignore
let parameters = message.message_data.into::<HistogramParameters>()?;
```

---

### Explicit Lifecycle

Every worker implements:

* `on_init()`
* `on_message()`
* `on_close()`

These lifecycle methods are mandatory.

---

### Centralized Error Handling

Errors returned by worker callbacks are automatically
captured by MWFErr.

Applications can monitor error events without requiring
individual workers to implement their own error reporting
mechanisms.

MWFErr can also be used by application code to record
custom errors, making it suitable as a centralized
error logging and monitoring system.

---

### Centralized Logging and Observability

MWF can log all message traffic, including:

* SenderID
* ReceiverID
* MessageID
* MessageInstanceID

This provides complete message traceability during development 
and production.

---

### Worker Type Registration

Before any worker instance can be spawned,
its worker type must be registered with MWF.

Each worker implementation defines its own unique
`WorkerTypeID` through the Worker trait.

Worker types are typically registered once during
application startup:

```rust,ignore
MWF::init()?;

MWF::register_worker_type::<CameraWorker>()?;
MWF::register_worker_type::<FFTWorker>()?;
MWF::register_worker_type::<DisplayWorker>()?;
```

Once registered, worker instances can be spawned normally.

This registration step allows MWF to associate each
`WorkerTypeID` with its corresponding Rust worker type.

Future MWF features, such as workflow-based worker
instantiation, will rely on this information.

---

## 4. Communication Patterns

All messaging functions are asynchronous.

MWF uses asynchronous backpressure.
When a worker queue reaches its configured capacity, message senders are suspended asynchronously until space becomes available. 
This prevents message loss while allowing the Tokio runtime to continue executing other workers.
Backpressure is transparent to the application and never blocks a Tokio worker thread.

### `send_message()`

Fire-and-forget message.

---

### `send_priority_message()`

Fire-and-forget message processed before normal messages.

Priority messages are handled using a Last-In, First-Out
(LIFO) policy.

---

### `send_message_and_wait()`

The current processing flow is suspended until the target
worker finishes processing the message or a timeout occurs.

The caller receives the processing result.

---

### Other Messaging Patterns

MWF also provides additional messaging patterns for more
specialized use cases:

* `send_priority_message_and_wait()`
* `send_unique_message()`

See the `MWFWrk` API documentation for the complete semantics
and usage of these messaging functions.

---

## 5. Error Handling

MWF provides a centralized error collection system.

Errors generated by worker callbacks are automatically
recorded by MWFErr.

Application code may also publish custom error events
through MWFErr::add_error().

Supported callback sources include:

* `on_init()`
* `on_message()`
* `on_close()`

Error events are stored in a circular buffer and can be
retrieved asynchronously by monitoring tools, loggers or
application code.

Each error event contains:

* ErrorInstanceID
* Timestamp
* ErrorSource/EntityID
* Error message
* Error source chain

---

## 6. Logging and Observability

MWF provides a centralized logging and observability system.

When logging is enabled, framework lifecycle events and message processing are recorded by MWFLog.

Supported framework events include:

* Worker spawn
* Worker initialization
* Worker shutdown
* Worker abort
* Message reception

Log events are stored in a circular buffer and can be retrieved asynchronously by monitoring tools, loggers or application code.

Each log event contains:

* LogInstanceID
* Timestamp
* WorkerID
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
                           MWF Runtime

                    ┌────────────────────┐
                    │        MWF         │
                    └─────────┬──────────┘
                              │
      ┌───────────────────────┼────────────────────────┐
      │                       │                        │
      ▼                       ▼                        ▼
┌────────────┐        ┌────────────┐          ┌────────────────┐
│   MWFWrk   │        │   MWFLog   │          │     MWFErr     │
├────────────┤        ├────────────┤          ├────────────────┤
│ Registry   │        │ Log buffer │          │ Error buffer   │
│ Messaging  │        │ Waiters    │          │ Waiters        │
│ Lifecycle  │        │ Filters    │          │                │
└─────┬──────┘        └────────────┘          └────────────────┘
      │
      ▼
┌──────────────────────────────────────────────────────────────┐
│                         WorkerTask                           │
│                                                              │
│  • owns the Worker instance                                  │
│  • executes lifecycle callbacks                              │
└──────────────────────────────┬───────────────────────────────┘
                               │ owns
                               ▼
                   ┌───────────────────────────┐
                   │        User Worker        │
                   │   (implements Worker)     │
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

                       MWF::init()
                             │
                             ▼
           MWF::register_worker_type::<CameraWorker>()
           MWF::register_worker_type::<FFTWorker>()
           MWF::register_worker_type::<DisplayWorker>()
                             │
                             ▼
            ──────────── Runtime ────────────
                             │
                             ▼
                MWF::spawn_worker(CameraWorker...)
                MWF::spawn_worker(FFTWorker...)
                MWF::spawn_worker(DisplayWorker...)
                             │
                             ▼
                    ┌──────────────────┐
                    │     MWFWrk       │
                    └────────┬─────────┘
                             │
                spawns one Tokio task per worker
                             │
                             ▼
         ┌─────────────┬───────────────┬───────────┐
         ▼             ▼               ▼
   +------------+ +------------+ +------------+
   | WorkerTask | | WorkerTask | | WorkerTask |
   +-----┬------+ +-----┬------+ +-------┬----+
         │              │                │
         ▼              ▼                ▼
   CameraWorker     FFTWorker      DisplayWorker
```

---

### Architecture main rule

Each worker owns its own mailbox and processes messages sequentially.

Parallelism is achieved by running multiple workers concurrently, not by processing multiple messages concurrently inside a single worker.

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

MWF is designed to be fully cross-platform and currently supports:

* Windows
* Linux

No platform-specific application code should be required when developing workers with MWF.

To help guarantee cross-platform compatibility, the repository includes a verification script (scripts/verify.ps1) that automatically validates the framework on both Windows and Linux (WSL2). The script performs formatting, linting, testing, example compilation, and documentation generation on each supported platform.

---

## 10. Quality Requirements

MWF includes automated tests.

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

MWF currently uses a global runtime instance.

Integration tests that call:

* `MWF::init()`
* `MWF::close()`

must not execute concurrently.

The project uses the `serial_test` crate and integration tests are annotated with:

```text
#[serial]
```

to ensure exclusive access to the MWF runtime during execution.

### Why are integration tests serialized?

MWF uses a global runtime instance stored in a static variable.

Running multiple integration tests concurrently could cause
Framework_AlreadyInitialized errors because several tests would
attempt to initialize the same MWF runtime simultaneously.


For this reason, integration tests using MWF are marked with
```text
#[serial]
```

---

## 13. Runtime overhead benchmark

MWF includes a runtime stress test that measures the framework overhead
without any application-specific processing.

Run it with:

```bash
cargo run --release --example runtime_overhead_benchmark
```

The number of workers can optionally be specified:

```bash
cargo run --release --example runtime_overhead_benchmark -- 100
cargo run --release --example runtime_overhead_benchmark -- 10000
```

The stress test measures:

- Worker creation
- Worker initialization
- Parallel `send_message_and_wait()`
- Worker shutdown
- Overall throughput

Example output:

```text
=====================================================
            MWF runtime overhead benchmark
=====================================================
Workers : 1000

Spawn                   :  4.784ms (4.78 µs/worker)
Init                    : 974.200µs (0.97 µs/worker)
SendAndWait (parallel)  :  1.406ms (1.41 µs/message)
Close                   :  3.979ms (3.98 µs/worker)

Total                   : 11.143ms
Throughput              : 711389 msg/s
=====================================================
```

Example benchmark output from one test environment.
Actual performance depends on hardware, Tokio configuration,
worker behavior and workload.

These measurements represent the runtime overhead of MWF using
`SimulationWorker` with `SimulationBehavior::DoNothing`.

---

## 14. Documentation

Additional design information is available in:

- Architecture.md

