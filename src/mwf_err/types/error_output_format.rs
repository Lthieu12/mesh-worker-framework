/// String output format.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ErrorOutputFormat {
    /// Human-readable fixed-width table.
    Terminal,
    /// Comma-separated values.
    CSV,
}
