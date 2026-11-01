use super::super::MWF;
use super::super::MWF_TIMESTAMP_FORMAT;
use super::super::TimestampFormat;
use chrono::Local;
use chrono::SecondsFormat;
use chrono::Utc;
use std::sync::atomic::Ordering;

impl MWF {
    /// Returns the current timestamp formatted according to RFC3339.
    ///
    /// The timestamp is generated using the current MWF timestamp
    /// configuration (local time or UTC).
    pub fn get_timestamp_rfc3339() -> String {
        match MWF_TIMESTAMP_FORMAT.load(Ordering::Relaxed) {
            x if x == TimestampFormat::LocalTime as u8 => Local::now().to_rfc3339_opts(SecondsFormat::Millis, false),
            x if x == TimestampFormat::UtcTime as u8 => Utc::now().to_rfc3339_opts(SecondsFormat::Millis, false),
            _ => Local::now().to_rfc3339_opts(SecondsFormat::Millis, false),
        }
    }
}
