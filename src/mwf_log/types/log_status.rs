/// Log execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogStatus {
    Start,
    Ok,
    Error,
}
