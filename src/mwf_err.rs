//! Public MWFErr (error handling) API.
//!
//! This API exposes the framework error handling
//! features.

mod functions;
mod types;

pub(crate) use types::ErrorData;
pub(crate) use types::terminal_error_column;
pub use types::{ErrorEvent, ErrorInstanceID, ErrorList, ErrorOutputFormat};

pub use types::MWFErr;
