# MWF Architecture

**Scope:** MWF 0.10.0

This document describes internal implementation details that are not
covered by the README or the public API documentation. It is intended
for maintainers and contributors.

It deliberately avoids restating MWF's purpose, public capabilities, or
application-facing usage patterns.

------------------------------------------------------------------------

## 1. Worker Type Registration and Instantiation

The worker type registry stores one entry per registered `WorkerTypeID`.
Each entry contains the Rust type name used for diagnostics and a
constructor function.

The constructor is a function pointer that creates a boxed worker
instance. Registration therefore stores a way to construct a worker; it
does not retain or create worker instances.

When `spawn_worker()` is called, MWF:

1.  Resolves the supplied `WorkerTypeID` in the type registry.
2.  Invokes the registered constructor to create a fresh worker
    instance.
3.  Creates the worker's normal and priority command channels.
4.  Spawns the `WorkerTask`, transferring ownership of the worker
    instance and the channel receivers to that task.
5.  Adds the worker's runtime entry to the worker registry, including
    its identifiers, parent, channel senders and task handle.

The type registry and worker registry serve different purposes: the
former maps type identifiers to constructors; the latter tracks
individual running worker instances.

## 2. WorkerTask and Execution

`WorkerTask` is the execution unit associated with one worker instance.
It owns the worker value and both command-channel receivers. The worker
registry retains the corresponding senders and the Tokio task handle.

The task establishes a worker execution context containing the current
`WorkerID` and parent `EntityID`. This context is available to MWF
operations called from within the worker and is maintained across
asynchronous suspension points.

When processing a message, the task also installs the message
inheritance context for the duration of the callback. This is the
mechanism used to associate messages sent from that callback with an
active MessageFlow.

The task collects commands received from its two channels into a local
`PendingCommands` queue. It then selects and executes commands
sequentially. A callback must finish before the task starts another
callback for the same worker.

A panic boundary surrounds the worker task future. A Rust panic is
caught at this boundary so MWF can record the failure and perform its
worker-task cleanup. This does not protect against process-level
failures such as access violations in native code.

## 3. Worker Lifecycle

The worker lifecycle is coordinated between the public worker-management
operations, the worker registry and the `WorkerTask`.

Spawning creates the task and registers its initial state as `Created`.
Initialization and shutdown are subsequently submitted as commands; the
registry state is updated as these operations are requested and as their
callbacks complete.

The lifecycle states represent both requested operations and callback
outcomes:

  State            Internal meaning
  ---------------- ---------------------------------------------------------
  `Created`        The worker task has been created and registered.
  `Initializing`   Initialization has been requested.
  `Initialized`    `on_init()` completed successfully.
  `InitFailed`     `on_init()` returned an error.
  `Closing`        Shutdown has been requested.
  `Closed`         `on_close()` completed successfully.
  `CloseFailed`    `on_close()` returned an error.
  `Panicked`       The worker task terminated through a caught Rust panic.

`InitFailed` and `CloseFailed` are callback outcomes, not equivalent to
a panic. A panic is handled by the task-level panic boundary rather than
by the normal callback result path.

On successful shutdown, the worker is removed from the registry. A
worker task that panics is also removed after its failure has been
reported. The registry therefore tracks active runtime entries, not a
persistent history of every worker that has existed.

## 4. Worker Registry and Hierarchy

The worker registry stores the runtime resources needed to address and
manage each worker. An entry contains the worker type, parent entity,
lifecycle state, normal and priority channel senders, and an optional
Tokio `JoinHandle`.

The task handle is optional because ownership can be transferred out of
the registry when an operation needs to await task termination.

Parent/child relationships are maintained separately from the worker
entries through a child-worker index keyed by parent `EntityID`. This
allows MWF to retrieve the workers attached to an entity without
scanning every registry entry.

A parent can be an external entity or another registered worker. When
the parent is a worker, it must already be registered and be in the
`Initialized` state before a child can be attached to it.

Removing a worker also updates the hierarchy index. A worker that has
been removed can no longer be used as a parent for newly spawned
children.

## 5. Mailboxes and Command Scheduling

Each worker has two bounded Tokio channels. Their receivers are owned by
its `WorkerTask`; their senders are retained by the worker registry.

The task drains available commands from both channels into
`PendingCommands` before selecting the next command. The pending queue
applies the scheduling rules below:

  Command type        Pending-queue policy
  ------------------- --------------------------------------
  Normal messages     FIFO
  Priority messages   LIFO
  Unique messages     FIFO, with replacement while pending

Priority commands are selected before pending normal commands. This is a
selection policy, not preemption: a callback already in progress runs to
completion before another command is selected.

For unique messages, the queue compares `MessageID` values. If a message
with the same identifier is already pending, its payload is replaced by
the new message without changing the existing queue position. The sender
identity is not part of this comparison.

The queue is local to the worker task. Scheduling therefore does not
require locking shared queue state and cannot cause callbacks on the
same worker instance to overlap.

## 6. Message Waiting and MessageFlow

The wait coordination subsystem maintains two kinds of runtime state:

-   Active MessageFlows, indexed by `MessageFlowID`, with the receiving
    worker and a one-shot reply sender.
-   Active worker waits, indexed by the waiting worker, with the
    receiver and message being awaited.

For a `WaitThisMessageFlow` request, MWF creates a flow identifier,
registers the flow against the target worker and sends the message with
that identifier in its inheritance context. The caller awaits the
corresponding one-shot reply.

When the receiving worker sends messages to itself while handling a
message with an active flow context, MWF propagates that context to
those self-directed messages. The context is not automatically
propagated to messages sent to other workers.

`message_flow_end()` completes the active flow associated with the current
message context. Completion validates that the flow belongs to the
calling worker, removes the flow from the registry and sends the
supplied business result to the waiting caller.

Flow completion is explicit. Returning an error from `on_message()` does
not by itself complete the flow. If the worker task terminates while
flows are still active, MWF removes those registrations and notifies
their callers that the worker task terminated.

To prevent deadlocks, MWF records worker-to-worker waits and checks
whether adding a new wait would create a cycle in the current wait
chain. Direct self-waits and detected cycles are rejected before the
wait is established.
