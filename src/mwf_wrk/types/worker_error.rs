/// Generic error type used by `on_init()`, `on_message()` and `on_close()`.
///
/// This is an alias to `anyhow::Error`, allowing workers to return any
/// error type implementing `std::error::Error`.
///
/// Most workers can simply use:
///
/// ```rust,ignore
/// return Err(anyhow!("Something went wrong"));
/// ```
pub type WorkerError = anyhow::Error;
