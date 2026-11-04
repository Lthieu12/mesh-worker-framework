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
- Parent/child creation relationships
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

    async fn on_init(&mut self, _init_data: MessageData) -> Result<(), WorkerError> {
        // Store identifiers for later use (for example to send messages or
        // spawn child workers from within the worker).
        self.worker_id = Some(MWFWrk::get_current_worker_id()?);
        self.parent_id = Some(MWFWrk::get_current_parent_id()?);

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

### Worker Creation Relationships

When a worker is spawned, `parent_id` identifies the entity that created it. This is a creation-origin reference, not an ownership or automatic lifecycle relationship: MWF does not automatically close or wait for workers when their creator terminates.

MWF provides `get_parent_id()` and `get_current_parent_id()` to retrieve this origin, and `get_children_ids()`, `get_children_count()`, `wait_children_init()`, `wait_children_close()` and `abort_children()` to inspect or explicitly manage workers created by an entity.

The creator must still be registered to use child-list operations. Surviving workers retain their original `parent_id` for diagnostics even after their creator has terminated.

---

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

MWF associates each `WorkerTypeID` with a constructor
capable of creating new worker instances.

This registration step allows MWF to associate each
`WorkerTypeID` with its corresponding Rust worker type.

---

## 4. Communication Patterns

All messaging functions are asynchronous.

MWF uses asynchronous backpressure.
When a worker queue reaches its configured capacity, message senders are suspended asynchronously until space becomes available.
This prevents messages from being dropped because of a full queue, while allowing the Tokio runtime to continue executing other workers.
Backpressure is transparent to the application and never blocks a Tokio worker thread.

### `send_message()`

Fire-and-forget message.

---

### `send_priority_message()`

Fire-and-forget message processed before pending normal messages.

Priority messages are handled using a Last-In, First-Out
(LIFO) policy.

---

### `send_message_and_wait()`

Send a message and wait for the requested completion mode:

* `WaitThisMessage`: wait until the target worker finishes its `on_message()` callback.
* `WaitThisMessageFlow`: wait until the target worker explicitly completes the message flow using `message_flow_end()`.

The timeout applies to the entire operation, including waiting for channel capacity and waiting for the requested completion.

A timeout does not cancel a message that has already been accepted by the worker. The worker continues processing it.

For `WaitThisMessageFlow`, a timeout after message admission does not cancel the MessageFlow. It remains active until the worker calls `message_flow_end()` or its WorkerTask terminates.

The worker must explicitly complete every MessageFlow, including when application-level processing fails.

The same timeout and completion semantics apply to `send_priority_message_and_wait()`.

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
through `MWFErr::add_error()`.

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

MWF provides a centralized logging and observability system through MWFLog.

The logging mode controls which worker events are captured:

* `NoLog`: disables worker event logging.
* `LogAll`: captures events from all workers.
* `LogSelected`: captures events only from the selected workers.

Captured log events are stored in an in-memory circular buffer and can be retrieved asynchronously by monitoring tools, loggers or application code.

### File Logging

File logging is independent of the logging mode.

The `log_file_path` configuration field determines whether a log file is created during MWF initialization. By default, MWF creates `mwf.log` next to the executable. Setting `log_file_path` to `None` prevents the file from being created.

File logging is disabled by default and can be enabled through `MWFLog::set_log_file_enabled(true)`.

When enabled, captured log events are written to the file and immediately flushed to disk.

If the configured maximum file size is reached or a file write operation fails, MWF automatically disables file logging and closes the file handle. In-memory logging continues normally.

File logging is not automatically restarted after this shutdown and cannot be re-enabled until MWF is initialized again.

### Log Event Content

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

Example Log Output
The following output is taken from `examples/getting_started_with_logging`:
```text
LogID        | Time         | WorkerID                       | Event   | MessageID                      | SenderID                       | P | W | STATUS | Duration
--------------------------------------------------------------------------------------------------------------------------------------------------------------------
000000000001 | 20:44:32.584 | DemoWorker:0:00001             | SPAWN   | -                              | -                              | - | - | OK     | 0ms
000000000002 | 20:44:32.585 | DemoWorker:0:00001             | INIT    | -                              | -                              | - | - | START  | -
000000000003 | 20:44:32.585 | DemoWorker:0:00001             | INIT    | -                              | -                              | - | - | OK     | 0ms
000000000004 | 20:44:32.585 | DemoWorker:0:00001             | MESSAGE | QUICK_CALCULATION              | MAIN                           | N | N | START  | -
000000000005 | 20:44:32.595 | DemoWorker:0:00001             | MESSAGE | QUICK_CALCULATION              | MAIN                           | N | N | OK     | 10ms
000000000006 | 20:44:32.595 | DemoWorker:0:00001             | MESSAGE | LONG_CALCULATION               | MAIN                           | N | N | START  | -
000000000007 | 20:44:33.596 | DemoWorker:0:00001             | MESSAGE | LONG_CALCULATION               | MAIN                           | N | N | OK     | 1000ms
000000000008 | 20:44:33.596 | DemoWorker:0:00001             | CLOSE   | -                              | -                              | - | - | START  | -
000000000009 | 20:44:33.596 | DemoWorker:0:00001             | CLOSE   | -                              | -                              | - | - | OK     | 0ms
```
`P` indicates whether the message has priority (`Y`) or is normal (`N`). `W` indicates whether the sender requested message completion (`Y`) or sent the message without waiting (`N`). `STATUS` reports the event state, and `Duration` shows the time taken by a completed operation. Timestamps and durations vary between runs.

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

MWF provides a consistent worker API across supported platforms, without requiring platform-specific code for framework operations.

To help guarantee cross-platform compatibility, the repository includes a verification script (`scripts/verify.ps1`) that automatically validates the framework on both Windows and Linux (WSL2). The script performs formatting, linting, testing, example compilation, and documentation generation on each supported platform.

---

## 10. Quality Requirements

The repository includes a verification script (`scripts/verify.ps1`)
that validates MWF on both Windows and Linux (WSL2).

The script performs the following checks on each platform:

* Formatting (`cargo fmt --check`)
* Linting (`cargo clippy` with warnings treated as errors)
* Unit and integration tests (`cargo test`)
* Documentation tests (`cargo test --doc`)
* Example compilation (`cargo build --examples`)
* Execution of the getting_started examples and runtime benchmark
* API documentation generation (`cargo doc`)

The verification process stops if any check fails.

---

## 11. Project Status

The core runtime is implemented and entering a stabilization phase.

Development now focuses primarily on reliability, testing,
documentation and production readiness, with targeted improvements
introduced when necessary.

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

Spawn                   :  4.733ms (4.73 µs/worker)
Init                    :  1.640ms (1.64 µs/worker)
SendAndWait (parallel)  :  1.816ms (1.82 µs/message)
Close                   :  3.326ms (3.33 µs/worker)

Total                   : 11.515ms
Throughput              : 550721 msg/s
=====================================================
```

Example benchmark output from one test environment.
Actual performance depends on hardware, Tokio configuration,
worker behavior and workload.

These measurements represent the runtime overhead of MWF using
`SimulationWorker` with `SimulationBehavior::DoNothing`.

---

## 14. Documentation

The project documentation includes:

- `README.md`: Getting started, core concepts and usage examples.
- `Architecture.md`: Architectural principles and runtime design.
- API documentation: Detailed reference for public types and functions.

---

## 15. License

This project is licensed under the MIT License.

