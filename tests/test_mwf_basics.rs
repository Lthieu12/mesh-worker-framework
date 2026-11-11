//! Validate MWF framework lifecycle and shutdown synchronization.
//!
//! These tests verify framework initialization,
//! shutdown, invalid lifecycle transitions and
//! terminal logger activation during shutdown.
use serial_test::serial;

#[cfg(test)]
#[serial]
mod tests {

    use mwf::*;
    use std::sync::Arc;
    use std::time::Duration;

    /// Verify that MWF rejects initialization without a Tokio runtime.
    #[test]
    fn test_mwf_init_without_tokio_runtime() {
        let result = MWF::init();

        assert!(matches!(result, Err(MWFError::Framework_InvalidTokioRuntime)));
    }

    /// Verify that MWF can be initialized and closed from a synchronous application
    /// using an explicitly created multi-threaded Tokio runtime.
    #[test]
    fn test_mwf_init_close() {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        // Verify that MWF can be used normally inside the explicit runtime.
        runtime.block_on(async {
            let info = MWF::get_info();
            println!("{}", info);

            assert!(!MWF::is_initialized());

            assert!(MWF::init().is_ok());

            assert!(MWF::is_initialized());

            assert!(MWF::close().is_ok());

            assert!(!MWF::is_initialized());
        });
    }

    /// Verify that MWF rejects multiple initialization attempts.
    #[test]
    fn test_mwf_double_init() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            assert!(MWF::init().is_ok());

            assert!(matches!(MWF::init(), Err(MWFError::Framework_AlreadyInitialized)));

            assert!(MWF::close().is_ok());
        });
    }

    /// Verify that MWF rejects closing when it is not initialized.
    #[test]
    fn test_mwf_double_close() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            assert!(matches!(MWF::close(), Err(MWFError::Framework_NotInitialized)));
        });
    }

    /// Verify that terminal logger activation is synchronized with framework shutdown.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_mwf_close_racing_with_terminal_logger_activation() {
        MWF::init().unwrap();

        // Synchronize both logger activation requests with framework shutdown.
        let start_barrier = Arc::new(tokio::sync::Barrier::new(3));

        let log_start_barrier = Arc::clone(&start_barrier);
        let log_task = tokio::spawn(async move {
            log_start_barrier.wait().await;
            MWFLog::set_log_terminal_enabled(true).await
        });

        let error_start_barrier = Arc::clone(&start_barrier);
        let error_task = tokio::spawn(async move {
            error_start_barrier.wait().await;
            MWFErr::set_error_terminal_enabled(true).await
        });

        // Release all three operations at approximately the same time.
        start_barrier.wait().await;

        // Shutdown is synchronous and may execute while both activation requests are pending.
        MWF::close().unwrap();

        // Both activation requests must eventually complete.
        let log_result = tokio::time::timeout(Duration::from_secs(2), log_task).await.expect("Terminal log activation did not complete").expect("Terminal log activation task panicked");

        let error_result = tokio::time::timeout(Duration::from_secs(2), error_task).await.expect("Terminal error activation did not complete").expect("Terminal error activation task panicked");

        // Activation may succeed before shutdown or be rejected after shutdown.
        assert!(matches!(&log_result, Ok(()) | Err(MWFError::Framework_NotInitialized)), "Unexpected terminal log activation result: {log_result:?}");

        assert!(matches!(&error_result, Ok(()) | Err(MWFError::Framework_NotInitialized)), "Unexpected terminal error activation result: {error_result:?}");

        assert!(!MWF::is_initialized());
    }
}
