use super::ErrorEvent;
use std::collections::VecDeque;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

/// Internal MWFErr storage.
///
/// ErrorData maintains a circular buffer containing
/// the most recent error events reported to MWFErr.
#[derive(Debug)]
pub(crate) struct ErrorData {
    /// Capacity of error event buffer.
    pub(crate) error_event_buffer_capacity: usize,

    /// Circular buffer of error events.
    pub(crate) error_event_list: VecDeque<ErrorEvent>,

    /// Terminal error logging task activation.
    pub(crate) error_terminal_enabled: bool,

    /// Terminal error logging task handle.
    pub(crate) error_terminal_task_handle: Option<JoinHandle<()>>,

    /// Terminal error logging task shutdown sender.
    pub(crate) error_terminal_shutdown_sender: Option<oneshot::Sender<()>>,
}
