//! Worker definitions for MWF.
//!
//! This file defines the worker trait and the types used
//! to identify and manage workers within the framework.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::message::{MessageData, WorkerMessage};
use crate::string_helper;

/// Function creating a new worker instance.
///
/// Each registered worker type provides a constructor
/// that MWF stores in the WorkerTypeRegistry.
///
/// Whenever MWF needs to spawn a worker from its
/// WorkerTypeID (for example when executing a workflow),
/// this constructor is invoked to create a fresh worker
/// instance.
///
/// The returned instance is then owned by the spawned
/// worker task.
pub(crate) type WorkerConstructor = fn() -> Box<dyn Worker + Send>;

/// Unique identifier of a worker type.
///
/// A WorkerTypeID identifies a worker *type*, not a worker instance.
///
/// Worker type identifiers are typically defined as compile-time
/// constants by each worker implementation, but they can also be
/// reconstructed at runtime when loading persisted workflow
/// definitions.
///
/// Rules:
/// - Must be unique across the whole application.
/// - Should remain stable between software versions if persisted.
/// - Is case-sensitive.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WorkerTypeID(String);

impl WorkerTypeID {
    /// Creates a worker type identifier.
    pub fn new(worker_type_id: impl Into<String>) -> Self {
        Self(worker_type_id.into())
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns a shortened version of the identifier.
    pub fn to_short_string(&self, max_len: usize) -> String {
        string_helper::to_short_string(self.as_str(), max_len)
    }

    // Validation only rejects identifiers that would break MWF itself
    // (logging, formatting, diagnostics, etc.).
    // Application-specific naming rules are intentionally left to the user.
    pub(crate) fn is_valid(&self) -> bool {
        let id = self.0.trim();
        !id.is_empty() && !id.contains('\n') && !id.contains('\r')
    }
}

impl std::fmt::Display for WorkerTypeID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for WorkerTypeID {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for WorkerTypeID {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for WorkerTypeID {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&WorkerTypeID> for WorkerTypeID {
    fn from(value: &WorkerTypeID) -> Self {
        value.clone()
    }
}

/// Unique identifier of an external entity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ExternalID(String);

impl ExternalID {
    /// Creates an external identifier.
    pub fn new(external_id: impl Into<String>) -> Self {
        Self(external_id.into())
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ExternalID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for ExternalID {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for ExternalID {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for ExternalID {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&ExternalID> for ExternalID {
    fn from(value: &ExternalID) -> Self {
        value.clone()
    }
}

/// Unique identifier of a worker instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct WorkerID(String);

impl WorkerID {
    /// Creates a worker identifier.
    pub fn new(worker_id: impl Into<String>) -> Self {
        Self(worker_id.into())
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns a shortened version of the identifier.
    pub fn to_short_string(&self, max_len: usize) -> String {
        string_helper::to_short_string(self.as_str(), max_len)
    }

    /// Returns this worker identifier as an EntityID.
    pub fn as_entity_id(&self) -> EntityID {
        EntityID::new_worker(self.clone())
    }
}

impl std::fmt::Display for WorkerID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for WorkerID {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for WorkerID {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for WorkerID {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&WorkerID> for WorkerID {
    fn from(value: &WorkerID) -> Self {
        value.clone()
    }
}

/// Generic error type used by `on_init()`, `on_message()` and `on_close()`.
///
/// This is an alias to `anyhow::Error`, allowing workers to return any
/// error type implementing `std::error::Error`.
///
/// Most workers can simply use:
///
/// ```rust,ignore
/// return Err(anyhow!("Something went wrong"));
/// ```
pub type WorkerError = anyhow::Error;

/// Identifier of an entity interacting with MWF.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityID {
    /// MWF worker entity.
    Worker(WorkerID),
    /// External entity not managed by MWF.
    External(ExternalID),
}

impl std::fmt::Display for EntityID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityID::Worker(worker_id) => {
                write!(f, "{worker_id}")
            }

            EntityID::External(external_id) => {
                write!(f, "External:{external_id}")
            }
        }
    }
}

impl EntityID {
    /// Creates a worker entity.
    pub fn new_worker(worker_id: WorkerID) -> Self {
        Self::Worker(worker_id)
    }

    /// Creates an external entity.
    pub fn new_external(external_id: impl Into<ExternalID>) -> Self {
        Self::External(external_id.into())
    }
    /// Returns a shortened human-readable representation of the entity.
    ///
    /// worker entities preserve the instance suffix whenever possible.
    /// External entities are truncated at the end.
    pub(crate) fn to_short_string(&self, max_len: usize) -> String {
        match self {
            EntityID::Worker(worker_id) => {
                let worker_id = worker_id.as_str();
                if worker_id.len() <= max_len {
                    return worker_id.to_string();
                }

                // Keep the instance suffix (":00001") visible.
                const SUFFIX_LEN: usize = 6;

                if max_len <= SUFFIX_LEN + 3 {
                    return worker_id[..max_len].to_string();
                }

                let prefix_len = max_len - SUFFIX_LEN - 3;

                format!("{}...{}", &worker_id[..prefix_len], &worker_id[worker_id.len() - SUFFIX_LEN..])
            }

            EntityID::External(text) => string_helper::to_short_string(text.as_str(), max_len),
        }
    }
}

/// Current state of a worker within the framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerState {
    /// worker has been created but not initialized.
    Created,
    /// worker initialization is in progress.
    Initializing,
    /// worker initialization has failed.
    InitFailed,
    /// worker initialization has completed successfully.
    Initialized,
    /// worker Closing is in progress.
    Closing,
    /// worker close has failed.
    CloseFailed,
    /// worker has been closed and is not usable anymore.
    Closed,
    /// worker has panicked and is not usable anymore.
    Panicked,
}

impl WorkerState {
    /// Returns a human-readable representation of the worker state.
    fn as_str(&self) -> &'static str {
        match self {
            WorkerState::Created => "Created",
            WorkerState::Initializing => "Initializing",
            WorkerState::InitFailed => "InitFailed",
            WorkerState::Initialized => "Initialized",
            WorkerState::Closing => "Closing",
            WorkerState::CloseFailed => "CloseFailed",
            WorkerState::Closed => "Closed",
            WorkerState::Panicked => "Panicked",
        }
    }
}

/// Current status of a worker within the framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerStatus {
    /// Current lifecycle state of the worker.
    pub worker_state: WorkerState,
    /// Number of normal messages currently waiting in the worker queue.
    pub normal_channel_size: usize,
    /// Maximum number of normal messages that can be queued by the worker.
    pub normal_channel_capacity: usize,
    /// Number of priority messages currently waiting in the worker queue.
    pub priority_channel_size: usize,
    /// Maximum number of priority messages that can be queued by the worker.
    pub priority_channel_capacity: usize,
}

/// Information describing a currently registered worker.
///
/// This structure provides a snapshot of the worker
/// state at the time it was retrieved.
///
/// It is primarily intended for:
/// - Debugging.
/// - Monitoring.
/// - User interfaces.
/// - Logging configuration tools.
#[derive(Debug, Clone)]
pub struct WorkerInfo {
    /// Unique identifier of the worker.
    pub worker_id: WorkerID,
    /// Identifier of the worker's parent entity.
    pub parent_id: EntityID,
    /// Current lifecycle state of the worker.
    pub worker_state: WorkerState,
}

impl WorkerInfo {
    /// Converts the worker information into a human-readable terminal string.
    fn to_terminal_string(&self) -> String {
        format!(
            "{:width_worker$} | {:width_father$} | {:width_state$}",
            EntityID::Worker(self.worker_id.clone()).to_short_string(terminal_worker_column::WORKER_ID_LEN,),
            self.parent_id.to_short_string(terminal_worker_column::FATHER_ID_LEN,),
            self.worker_state.as_str(),
            width_worker = terminal_worker_column::WORKER_ID_LEN,
            width_father = terminal_worker_column::FATHER_ID_LEN,
            width_state = terminal_worker_column::STATE_LEN,
        )
    }

    /// Converts the worker information into a CSV row.
    fn to_csv_string(&self) -> String {
        format!("{},{},{}", string_helper::csv_escape(self.worker_id.as_str()), string_helper::csv_escape(&self.parent_id.to_string()), string_helper::csv_escape(self.worker_state.as_str()),)
    }
}

/// Output format used to represent worker information.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WorkerOutputFormat {
    /// Human-readable terminal output.
    Terminal,
    /// Comma-separated values output.
    CSV,
}

/// Terminal worker column widths.
mod terminal_worker_column {

    pub(crate) const WORKER_ID_LEN: usize = 30;
    pub(crate) const FATHER_ID_LEN: usize = 30;
    pub(crate) const STATE_LEN: usize = 14;
}

/// List of worker information returned by MWFWrk.
#[derive(Debug, Clone)]
pub struct WorkerList {
    /// Worker information entries contained in the list.
    pub worker_info_list: Vec<WorkerInfo>,
}

impl std::fmt::Display for WorkerList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        writeln!(f, "{}", Self::get_header(WorkerOutputFormat::Terminal))?;

        for line in self.to_string_list(WorkerOutputFormat::Terminal) {
            writeln!(f, "{line}")?;
        }
        Ok(())
    }
}

impl WorkerList {
    /// Returns the column header corresponding to the requested format.
    pub fn get_header(format: WorkerOutputFormat) -> String {
        match format {
            WorkerOutputFormat::Terminal => {
                let header = format!("{:<width_worker$} | {:<width_father$} | {:<width_state$}", "WorkerID", "ParentID", "State", width_worker = terminal_worker_column::WORKER_ID_LEN, width_father = terminal_worker_column::FATHER_ID_LEN, width_state = terminal_worker_column::STATE_LEN,);

                let separator = "-".repeat(header.len());

                format!("{header}\n{separator}")
            }

            WorkerOutputFormat::CSV => String::from("WorkerID,ParentID,State"),
        }
    }

    /// Converts a worker information list into strings.
    pub fn to_string_list(&self, format: WorkerOutputFormat) -> Vec<String> {
        let mut worker_info_list = self.worker_info_list.clone();
        worker_info_list.sort_by(|a, b| a.worker_id.cmp(&b.worker_id));

        match format {
            WorkerOutputFormat::Terminal => worker_info_list.iter().map(|worker_info| worker_info.to_terminal_string()).collect(),

            WorkerOutputFormat::CSV => worker_info_list.iter().map(|worker_info| worker_info.to_csv_string()).collect(),
        }
    }
}

/// Interface implemented by all MWF workers.
///
/// MWF creates worker instances dynamically by calling
/// `worker::new()` on the registered worker type.
///
/// MWF stores worker instances behind `Box<dyn Worker>`.
///
/// The `Self: Sized` bounds on `worker_type_id()` and `new()` are
/// required to keep this trait object-safe. Without these bounds,
/// `worker` could no longer be used as `dyn Worker`.
///
#[async_trait]
pub trait Worker {
    /// Returns the unique identifier of this worker type.
    ///
    /// Every worker implementation must provide a unique WorkerTypeID.
    ///
    /// This associated function is only available on concrete worker
    /// types (`Self: Sized`). It is intentionally excluded from trait
    /// objects (`dyn Worker`) so that MWF can store workers behind
    /// `Box<dyn Worker>`.
    fn worker_type_id() -> WorkerTypeID
    where
        Self: Sized;

    /// Creates a new worker instance.
    ///
    /// MWF calls this function whenever a new worker
    /// instance must be spawned from its registered
    /// WorkerTypeID.
    ///
    /// The returned instance must be in its initial state.
    /// Runtime resources should be acquired later during
    /// on_init().
    ///
    /// This associated function is only available on concrete worker
    /// types (`Self: Sized`). It is intentionally excluded from trait
    /// objects (`dyn worker`) so that MWF can store workers behind
    /// `Box<dyn worker>`.
    fn new() -> Self
    where
        Self: Sized;

    /// Called once when the worker is initialized.
    async fn on_init(&mut self, init_data: MessageData) -> Result<(), WorkerError>;

    /// Called when a message is received by the worker.
    async fn on_message(&mut self, message: WorkerMessage) -> Result<(), WorkerError>;

    /// Called once before the worker is closed.
    async fn on_close(&mut self, close_data: MessageData) -> Result<(), WorkerError>;
}
