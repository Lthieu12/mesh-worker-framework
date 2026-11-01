/// Logging mode.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogMode {
    /// Logging disabled.
    NoLog = 0,
    /// Log all workers.
    LogAll = 1,
    /// Log only selected workers.
    LogSelected = 2,
}
