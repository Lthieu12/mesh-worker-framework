//! Global state owned by the MWF runtime.

use std::sync::atomic::{AtomicU8, AtomicU64};
use std::sync::{LazyLock, RwLock};
use tokio::sync::Notify;

use super::types::{MWFData, TimestampFormat};
use crate::mwf_err::ErrorData;
use crate::mwf_log::{LogMode, LoggingData};
use crate::mwf_wrk::WorkerData;

/// Global MWF instance counter.
pub(crate) static MWF_INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Global MWF runtime storage shared by all
/// framework components.
pub(crate) static MWF_DATA: LazyLock<RwLock<Option<MWFData>>> = LazyLock::new(|| RwLock::new(None));

/// Global MWF worker runtime storage.
pub(crate) static MWF_WRK_DATA: LazyLock<RwLock<Option<WorkerData>>> = LazyLock::new(|| RwLock::new(None));

/// Global MWF notifier used to signal child worker initialized.
pub(crate) static MWF_WRK_INIT_NOTIFY: LazyLock<Notify> = LazyLock::new(Notify::new);

/// Global MWF notifier used to signal child worker closures.
pub(crate) static MWF_WRK_CLOSE_NOTIFY: LazyLock<Notify> = LazyLock::new(Notify::new);

/// Global MWF message instance counter used to identify messages sent to workers.
pub(crate) static MWF_WRK_MESSAGE_INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Global MWF message flow counter used to identify message flows.
pub(crate) static MWF_WRK_MESSAGE_FLOW_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Global MWF error event storage.
pub(crate) static MWF_ERR_DATA: LazyLock<RwLock<Option<ErrorData>>> = LazyLock::new(|| RwLock::new(None));

/// MWF error notification emitted whenever a new error event
/// is added to MWFErr.
pub(crate) static MWF_ERR_NOTIFY: LazyLock<Notify> = LazyLock::new(Notify::new);

/// Global MWF error instance counter.
pub(crate) static MWF_ERR_INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Global logging storage.
pub(crate) static MWF_LOG_DATA: LazyLock<RwLock<Option<LoggingData>>> = LazyLock::new(|| RwLock::new(None));

/// Current logging mode.
pub(crate) static MWF_LOG_MODE: AtomicU8 = AtomicU8::new(LogMode::NoLog as u8);

/// MWF log notification emitted whenever a new log event
/// is added to MWFLog.
pub(crate) static MWF_LOG_NOTIFY: LazyLock<Notify> = LazyLock::new(Notify::new);

/// Global log event counter.
pub(crate) static MWF_LOG_INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Current timestamp format used by the framework.
pub(crate) static MWF_TIMESTAMP_FORMAT: AtomicU8 = AtomicU8::new(TimestampFormat::LocalTime as u8);
