# Mesh Module Framework (MMF)

Mesh Module Framework (MMF) is a Rust Tokio-based modular runtime designed for applications composed of independent communicating modules.

MMF provides:

* Module lifecycle management
* Inter-module messaging
* Broadcast messaging
* Centralized logging
* Parent/child hierarchy management
* Reply handling
* Message prioritization
* Cross-platform support (Windows and Linux)

---

## Core Principles

### One Module = One Message at a Time

A module never processes multiple messages simultaneously.

This guarantees deterministic behavior and simplifies communication with hardware devices, external APIs and long-running calculations.

Parallelism is achieved by running multiple modules concurrently, not by processing multiple messages concurrently inside a single module.

---

### Framework-Agnostic MessageDatas

MMF transports opaque MessageDatas (`Bytes`).

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

### Centralized Observability

MMF can log all message traffic, including:

* SenderID
* ReceiverID
* MessageID
* MessageInstanceID

This allows runtime debugging in both development and production environments.

---

## Communication Patterns

### Send

Fire-and-forget message.

No reply is expected.

---

### SendAndAskReply

A reply is requested.

The sender continues its normal execution.

The reply will later be received and processed as a normal message.

---

### SendAndWaitReply

A reply is requested.

The current processing flow is suspended until the expected reply is received or a timeout occurs.

The reply is considered part of the current logical sequence and must be available immediately to continue processing.

---

## Architecture Overview

```text
MMF Runtime
│
├── Module A
├── Module B
├── Module C
└── Module D
```

Each module owns its own mailbox and processes messages sequentially.

---

## Design Goals

* Easy integration
* Minimal boilerplate
* Cross-platform support
* Deterministic behavior
* Centralized logging
* Runtime observability
* Automated testing

---

## Platform Support

* Windows
* Linux

No platform-specific application code should be required.

---

## Quality Requirements

MMF includes automated tests.

A build must fail if a test fails.

The long-term objective is to validate:

* Compilation
* Unit tests
* Integration tests
* Windows compatibility
* Linux compatibility

---

## Project Status

Early design phase.

---

## Running tests

MMF currently uses a global runtime instance.

Integration tests that call:

- MMF::init()
- MMF::close()

must not execute concurrently.

The project uses the `serial_test` crate and integration tests are annotated with:

#[serial]

to ensure exclusive access to the MMF runtime during execution.

### Why are integration tests serialized?

MMF uses a global runtime instance stored in a static variable.

Running multiple integration tests concurrently could cause
Framework_AlreadyInitialized errors because several tests would
attempt to initialize the same MMF runtime simultaneously.

For this reason, integration tests using MMF are marked with
#[serial].
