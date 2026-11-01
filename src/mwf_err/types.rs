//! Error event, buffer, and configuration types.

mod error_data;
mod error_event;
mod error_instance_id;
mod error_list;
mod error_output_format;
mod mwf_err;
pub(crate) mod terminal_error_column;

pub(crate) use error_data::ErrorData;
pub use error_event::ErrorEvent;
pub use error_instance_id::ErrorInstanceID;
pub use error_list::ErrorList;
pub use error_output_format::ErrorOutputFormat;
pub use mwf_err::MWFErr;
