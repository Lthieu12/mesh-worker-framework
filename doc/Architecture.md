# MMF Architecture

## Purpose

Mesh Module Framework (MMF) is a Tokio-based modular runtime for applications composed of independent communicating modules.

MMF provides runtime services such as:

* Module lifecycle management
* Message routing
* Parent/child hierarchy management
* Centralized error collection
* Runtime observability

MMF is intentionally application-agnostic.

Application-specific concepts such as workflows, measurements, calculations or business logic are expected to be implemented by modules built on top of MMF.

Example:

```text
MMF
 ├── CameraModule
 ├── FFTModule
 ├── LoggerModule
 └── WorkflowModule
```

All modules are considered application code. MMF itself remains unaware of their responsibilities.

---

# Runtime Model

MMF uses one Tokio task per module.

After a module is spawned:

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
  │   MMFMod   │      │   MMFLog   │       │   MMFErr   │
  └─────┬──────┘      └────────────┘       └────────────┘
        │
        │ owns
        ▼
  ┌──────────────────────────────────────────────┐
  │                 ModuleTask                   │
  └──────────────────┬───────────────────────────┘
                     │ owns
                     ▼
              User Module
          (implements Module)
```

MMF owns:

* Communication channels
* JoinHandles
* Runtime metadata

ModuleTask owns:

* The module instance
* Message processing
* Lifecycle callback execution

MMF never directly calls:

```rust
on_init()

on_message()

on_close()
```

These callbacks are always executed inside the Tokio task that owns the module instance.

---

# Module

A Module is the fundamental execution unit of MMF.

Every module owns:

* A unique ModuleID
* A mailbox
* A Tokio task
* A lifecycle state

Every module implements:

```rust
on_init()

on_message()

on_close()
```

These lifecycle methods are mandatory.

```text
              User Application

        ex: AcquisitionModule
        ex: FFTModule
        ex: DisplayModule
             │
             ▼
     implements Module trait

             │
             ▼
        MMF::spawn_module()

             │
             ▼
        creates ModuleTask

             │
             ▼
      executes callbacks

        on_init()

        on_message()

        on_close()
```

---

# Execution Model

A module processes one event at a time.

```text
                 ModuleTask

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

No two callbacks can execute simultaneously on the same module instance.

Parallelism is achieved by running multiple modules concurrently.

This guarantees deterministic module behavior and eliminates the need for internal synchronization inside most modules.

---

# Module Lifecycle

Modules transition through explicit lifecycle states.

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
User Module A
      │
      │ send_message()
      ▼
+-------------+
|   MMFMod    |
+-------------+
      │
      │ route message
      ▼
+-------------+
| ModuleTask  |
+-------------+
      │
      │ execute callback
      ▼
+--------------+
| User Module B|
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

The sender waits until the target module completes message processing or a timeout occurs.

The processing result is returned to the caller.

---

# Message Priorities

MMF maintains two independent channels per module:

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

Modules may declare a parent entity.

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

MMF maintains a runtime registry containing information about active modules.

Typical information includes:

* ModuleID
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
