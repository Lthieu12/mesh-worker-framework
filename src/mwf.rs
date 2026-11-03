//! Public MWF API.
//!
//! This API exposes the framework entry points used to
//! initialize and shutdown the API global runtime.

mod framework_error;
mod functions;
mod state;
mod types;

pub use framework_error::MWFError;
pub(crate) use framework_error::MWFPanic;

pub(crate) use state::{
    MWF_DATA, MWF_ERR_DATA, MWF_ERR_INSTANCE_COUNTER, MWF_ERR_NOTIFY, MWF_INSTANCE_COUNTER, MWF_LOG_DATA, MWF_LOG_INSTANCE_COUNTER, MWF_LOG_MODE, MWF_LOG_NOTIFY, MWF_TIMESTAMP_FORMAT, MWF_WRK_CLOSE_NOTIFY, MWF_WRK_DATA, MWF_WRK_INIT_NOTIFY, MWF_WRK_MESSAGE_FLOW_COUNTER,
    MWF_WRK_MESSAGE_INSTANCE_COUNTER, MWF_WRK_MESSAGE_WAIT_DATA,
};
pub use types::{MWFConfiguration, MWFInfo, TimestampFormat};
pub(crate) use types::{MWFData, MWFInstanceID};

pub use types::MWF;
