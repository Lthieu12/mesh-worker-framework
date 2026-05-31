# MMF Architecture

## Purpose

Mesh Module Framework (MMF) is a messaging and runtime framework.

MMF is not a workflow engine.

Workflow orchestration is expected to be implemented by application-specific modules built on top of MMF.

Example:

```text
MMF
 ├── CameraModule
 ├── FFTModule
 ├── LoggerModule
 └── WeftHandler
```

In this example, `WeftHandler` is a normal MMF module and not part of the framework core.

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

---

# Execution Model

One module processes one message at a time.

```text
on_init()

↓

Message A

↓

Message B

↓

Message C

↓

on_close()
```

No two messages can be processed simultaneously by the same module.

Parallelism is achieved through multiple modules running concurrently.

---

# Messages

Messages are transported using a generic framework message.

Conceptually:

```text
SenderID
ReceiverID
MessageID
MessageInstanceID
MessageData
```

MessageDatas are transported as `Bytes`.

MMF never interprets application data.

---

# Message Routing

MMF supports:

* Private messages
* Broadcast messages

Broadcast messages are delivered to all subscribed modules.

---

# Parent / Child Hierarchy

Modules may declare a parent module.

Example:

```text
Parent
 ├── Child A
 └── Child B
```

During shutdown, a parent waits for its children to terminate before terminating itself.

A timeout mechanism prevents deadlocks.

---

# Reply Modes

MMF distinguishes three communication patterns.

## Send

No reply expected.

---

## SendAndAskReply

A reply is expected.

The sender continues processing normally.

The reply is later processed as a standard message.

---

## SendAndWaitReply

A reply is expected.

The sender suspends the current processing sequence until the expected reply is received or a timeout occurs.

The reply is considered part of the current logical sequence and must be available immediately to continue processing.

This behavior is functionally different from `SendAndAskReply`.

---

# Message Priorities

MMF supports prioritized messages.

Priorities are intended for execution ordering requirements.

Example:

```text
Measurement A

↓

Step1

↓

Step2

↓

Step3
```

A module may use priority messages to ensure that Measurement A is fully processed before Measurement B begins.

The exact implementation of priority handling is intentionally left open at this stage.

---

# Logging

MMF includes centralized logging.

The framework must be capable of tracing message traffic at runtime.

Typical log information:

```text
Timestamp
SenderID
ReceiverID
MessageID
MessageInstanceID
```

Logging must remain available in production executables and not only during development.

---

# Registry

MMF maintains a runtime registry containing information about active modules.

Typical information:

* ModuleID
* ParentModuleID
* State
* Startup status

This registry is used for routing, lifecycle management and shutdown coordination.

---

# Serialization

MMF transports opaque MessageDatas.

The framework is intentionally unaware of application-specific data structures.

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

MMF must include automated tests.

A build must fail if tests fail.

The project should eventually include:

* Unit tests
* Integration tests
* Example-based tests
* Windows CI validation
* Linux CI validation

---

# Open Design Topics

The following topics remain under active design:

* Mailbox implementation
* Priority message implementation
* Reply handling implementation
* Runtime internals
* Logging backend selection
* Registry internals
