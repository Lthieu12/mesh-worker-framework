//! Validate MWF framework lifecycle operations.
//!
//! These tests verify framework initialization,
//! shutdown and protection against invalid
//! lifecycle transitions.
use serial_test::serial;

#[cfg(test)]
#[serial]
mod tests {

    use mwf::*;

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
}
