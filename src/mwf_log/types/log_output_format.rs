/// String output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogOutputFormat {
    /// Human-readable fixed-width table.
    Terminal,
    /// Comma-separated values.
    CSV,
}
