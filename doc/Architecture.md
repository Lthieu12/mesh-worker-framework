# MMF Architecture

## Purpose

Mesh Worker Framework (MMF) is a Tokio-based modular runtime for applications composed of independent communicating workers.

MMF provides runtime services such as:

* Worker lifecycle management
* Message routing
* Parent/child hierarchy management
* Centralized error collection
* Runtime observability

MMF is intentionally application-agnostic.

Application-specific concepts such as workflows, measurements, calculations or business logic are expected to be implemented by workers built on top of MMF.

Example:

```text
MMF
 ├── CameraWorker
 ├── FFTWorker
 ├── LoggerWorker
 └── WorkflowWorker
```

All workers are considered application code. MMF itself remains unaware of their responsibilities.

---

# Design Principles

MMF intentionally makes several architectural choices that may differ from traditional actor frameworks.

These decisions are intended to keep the runtime deterministic, application-agnostic and easy to reason about.

## Registered Worker Types

Workers are registered once during application startup.

```rust
MMF::register_worker_type::<CameraWorker>()?;
```

MMF associates each WorkerTypeID with a constructor capable of creating new worker instances.

Workers are then spawned using their WorkerTypeID rather than concrete Rust types.

```rust
let worker_id = MMFWrk::spawn_worker("CameraWorker", &EntityID::External("MAIN".into()))?;
```

This also decouples application configuration from Rust code. Workflows can be loaded from configuration files, 
databases or graphical editors without embedding Rust types into application data.

---

## Explicit Lifecycle

Creating a worker and initializing it are two independent operations.

```text
spawn_worker()

↓

init_worker()

↓

on_init()
```

Separating creation from initialization gives the application complete control over the worker startup sequence.

For example, multiple workers can be spawned first, then initialized simultaneously or in a specific order.

Workers can therefore be created before the resources they require become available.

---

## Opaque Binary Data

MMF never interprets application data.

Both lifecycle data and message data are transported as opaque Bytes.

MMF does not require or recommend any particular serialization format.

```rust
MMFWrk::init_worker(worker_id, lifecycle_data).await?;

MMFWrk::send_message(..., message_data).await?;
```

Serialization and deserialization remain entirely under application control.

This keeps MMF independent from any serialization library or application-specific data model.

---

## One Message at a Time

Each worker processes a single message at a time.

Callbacks are never executed concurrently on the same worker instance.

This greatly simplifies stateful worker development, since most workers can safely mutate their internal state without additional synchronization primitives.

Parallelism is achieved by running multiple workers concurrently rather than processing multiple messages simultaneously inside a single worker.

---

## Explicit Observability

MMF automatically records worker lifecycle events, message processing and framework errors.

Applications can inspect these events through MMFLog and MMFErr without instrumenting their own workers.

This makes runtime diagnostics available by default while keeping application code focused on business logic.

---

# Runtime Model

MMF uses one Tokio task per worker.

After a worker is spawned:

```text
                           MMF Runtime

                    ┌────────────────────┐
                    │        MMF         │
                    └─────────┬──────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
        ▼                     ▼                     ▼
  ┌────────────┐      ┌────────────┐       ┌────────────┐
  │   MMFWrk   │      │   MMFLog   │       │   MMFErr   │
  └─────┬──────┘      └────────────┘       └────────────┘
        │
        │ owns
        ▼
  ┌──────────────────────────────────────────────┐
  │                 WorkerTask                   │
  └──────────────────┬───────────────────────────┘
                     │ owns
                     ▼
              User Worker
          (implements Worker)
```

MMF owns:

* Communication channels
* JoinHandles
* Runtime metadata

WorkerTask owns:

* The worker instance
* Message processing
* Lifecycle callback execution

MMF never directly calls:

```rust
on_init()

on_message()

on_close()
```

These callbacks are always executed inside the Tokio task that owns the worker instance.

---

# Worker

A Worker is the fundamental execution unit of MMF.

Every worker owns:

* A unique WorkerID
* A mailbox
* A Tokio task
* A lifecycle state

Every worker implements:

```rust
on_init()

on_message()

on_close()
```

These lifecycle methods are mandatory.

```text
              User Application

        ex: AcquisitionWorker
        ex: FFTWorker
        ex: DisplayWorker
             │
             ▼
     implements Worker trait

             │
             ▼
        MMF::spawn_worker()

             │
             ▼
        creates WorkerTask

             │
             ▼
      executes callbacks

        on_init()

        on_message()

        on_close()
```

---

# Execution Model

A worker processes one event at a time.

```text
                 WorkerTask

                    │

               on_init()

                    │

         +----------+----------+
         |                     |
         ▼                     ▼

    Priority Queue        Normal Queue

         │                     │
         └──────────┬──────────┘
                    ▼

             on_message()

                    │

             on_message()

                    │

               on_close()
```

No two callbacks can execute simultaneously on the same worker instance.

Parallelism is achieved by running multiple workers concurrently.

This guarantees deterministic worker behavior and eliminates the need for internal synchronization inside most workers.

---

# Worker Lifecycle

Workers transition through explicit lifecycle states.

```text
Created

↓

Initializing

↓

Initialized

↓

Closing

↓

Closed
```

Failure states:

```text
InitFailed

CloseFailed
```

Lifecycle transitions are controlled by MMF and not by application code.

---

# Messaging

Messages are transported through a framework-defined structure.

Conceptually:

```text
SenderID
ReceiverID
MessageID
MessageInstanceID
MessageData
```

```text
User Worker A
      │
      │ send_message()
      ▼
+-------------+
|   MMFWrk    |
+-------------+
      │
      │ route message
      ▼
+-------------+
| WorkerTask  |
+-------------+
      │
      │ execute callback
      ▼
+--------------+
| User Worker B|
| on_message() |
+--------------+
```

MessageData is transported as:

```rust
Bytes
```

MMF never interprets application data.

Serialization and deserialization remain the responsibility of the application.

---

# Communication Patterns

## Send

Fire-and-forget message.

The sender does not wait for message processing.

---

## SendPriority

Fire-and-forget message processed before normal messages.

Priority messages use a Last-In First-Out (LIFO) policy.

---

## SendAndWait

The sender waits until the target worker completes message processing or a timeout occurs.

The processing result is returned to the caller.

---

# Message Priorities

MMF maintains two independent channels per worker:

```text
Normal Channel
Priority Channel
```

Priority messages are always processed before normal messages.

Priority messages are internally stored in a stack and executed using a LIFO policy.

Example:

```text
SendPriority(A)
SendPriority(B)
SendPriority(C)

↓

C
B
A
```

---

# Parent / Child Hierarchy

Workers may declare a parent entity.

Example:

```text
Parent
 ├── Child A
 └── Child B
```

MMF maintains this hierarchy through the Registry.

The hierarchy is primarily used for coordinated shutdown operations.

MMF provides APIs allowing a parent to wait until all direct children have terminated.

---

# Registry

MMF maintains a runtime registry containing information about active workers.

Typical information includes:

* WorkerID
* Parent EntityID
* Lifecycle state
* Communication channels
* JoinHandle ownership information

The Registry is used for:

* Message routing
* Lifecycle management
* Hierarchy management
* Shutdown coordination

---

# Error Handling

MMF provides centralized error collection through MMFErr.

Errors returned by:

```rust
on_init()

on_message()

on_close()
```

are automatically recorded.

Each error event contains:

```text
ErrorInstanceID
Timestamp
SenderID
ErrorMessage
ErrorSourceChain
```

Error events are stored in a circular buffer.

Applications may retrieve error events through MMFErr APIs or publish their own error events using:

```rust
MMFErr::add_error(...)
```

This allows framework and application errors to share the same monitoring infrastructure.

---

# Serialization

MMF transports opaque MessageData values.

The framework is intentionally unaware of application-specific structures.

Example:

```rust
Measurement::to_bytes()

Measurement::from_bytes()
```

Serialization and deserialization remain the responsibility of the application.

---

# Portability

MMF must compile and run on:

* Windows
* Linux

with minimal user configuration.

---

# Quality Requirements

MMF includes automated tests.

A build must fail if tests fail.

The project includes:

* Unit tests
* Integration tests

Additional validation may be added in the future:

* Windows CI validation
* Linux CI validation
* Example-based tests

```
```
