/// Timestamp format used by MWF.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimestampFormat {
    LocalTime = 0,
    UtcTime = 1,
}
