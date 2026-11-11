# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.11.0] - 2026-11-11

### Added

- Add MWF::get_configuration() to retrieve a copy of the configuration used to initialize MWF.
- Add the crate license to MWFInfo returned by MWF::get_info().
- Include the complete getting_started example in the Rust documentation of MWF::init(), directly sourcing the example files.

### Changed

#### Breaking API changes

Add the license field to the public MWFInfo structure. This may require code changes for applications that construct MWFInfo directly or destructure all its fields. Applications using MWF::get_info() without constructing MWFInfo remain unaffected.

## [0.10.0] - 2026-11-04

### Added

#### MessageFlow and messaging

- Add `MessageWaitMode` with `WaitThisMessage` and `WaitThisMessageFlow`.
- Add `MWFWrk::message_flow_end()` to explicitly complete a MessageFlow.
- Propagate MessageFlow context across chained messages sent to the same worker.
- Track pending waits to prevent self-waits, concurrent waits from the same worker, and circular wait deadlocks.
- Include detected wait chains in deadlock errors.
- Clean up active MessageFlows when a `WorkerTask` terminates.

#### Worker lifecycle and execution context

- Add task-local worker execution context.
- Add `MWFWrk::get_current_worker_id()` and `MWFWrk::get_current_parent_id()`.
- Reject attempts by a worker to initialize, close, or wait for itself.
- Reject MWF initialization and shutdown calls from worker execution contexts.
- Validate worker execution contexts before accepting framework API operations.

#### Worker management and API improvements

- Validate `WorkerTypeID`, `ExternalID`, `MessageID`, and `EntityID` values.
- Add public identifier validation methods.
- Derive `Serialize` and `Deserialize` for `EntityID`.
- Add explicit `MWFError` variants for invalid execution contexts, self-management, self-waits, and deadlock detection.

#### Examples, tests, and documentation

- Add a MessageFlow example with logging.
- Update existing examples and simulation workers.
- Expand tests for MessageFlow completion, timeout handling, worker context, deadlock prevention, lifecycle management, panic cleanup, and logging failures.
- Improve the README and architecture documentation.
- Update Windows and Linux verification scripts.
- Add MIT license information and package documentation metadata.

### Changed

#### Breaking API changes

- `Worker::on_init()` now receives only `MessageData`. Use `MWFWrk::get_current_worker_id()` and `MWFWrk::get_current_parent_id()` to retrieve the current worker context.
- `MWFWrk::send_message_and_wait()` and `MWFWrk::send_priority_message_and_wait()` now require a `MessageWaitMode` argument.
- `MWFLog::wait_log()` now returns `Result<Option<LogList>, MWFError>`.
- `MWFErr::wait_error()` now returns `Result<Option<ErrorList>, MWFError>`.
- Worker list APIs now accept `&[WorkerID]` instead of `&Vec<WorkerID>`.
- New `MWFError` variants may require updates to exhaustive match expressions.

#### Runtime behavior and compatibility

- Extend send-and-wait timeouts to cover both channel admission and message completion.
- Keep messages accepted by a receiver and active MessageFlows running when the caller times out.
- Allow newly received priority messages to be processed before pending normal messages.
- Ignore duplicate `WorkerID` values in worker list operations, processing each unique worker only once.
- Make `abort_worker_list()` a best-effort operation: continue processing the remaining workers after an error and return the first encountered error after processing the list.
- Include the MWF instance ID in generated `WorkerID` values to prevent collisions across successive initialization cycles. The format is now `WorkerTypeID:MWFInstanceID:WorkerCounter`.
- Bind pending log and error waits to the MWF instance in which they were started.
- Return `Ok(None)` when pending log or error waits are released by framework shutdown.
- Set Rust 1.85 as the minimum supported version.
- Restrict enabled Tokio features to those required by MWF.

### Fixed

- Prevent logging failures from interrupting worker execution or spawning.
- Preserve in-memory log events when file writing fails.
- Automatically disable file logging when a write fails or the configured maximum file size is reached.
- Prevent file logging from being re-enabled after its file handle has been closed.
- Improve worker task cleanup and panic handling.
- Preserve late log and error events from aborted workers for diagnostics, retaining their original `WorkerID` values.

### Removed

- Remove the unused `bincode` dependency.

This release includes a small number of breaking API changes. The required updates should generally be straightforward: adjust worker initialization and send-and-wait calls, and account for new MWFError variants in exhaustive matches where applicable.
