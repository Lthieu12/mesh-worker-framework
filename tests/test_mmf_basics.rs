//! Validate MWF framework lifecycle operations.
//!
//! These tests verify framework initialization,
//! shutdown and protection against invalid
//! lifecycle transitions.
use serial_test::serial;

#[cfg(test)]
#[serial]
mod tests {

    use mesh_worker_framework::*;

    #[test]
    fn test_mmf_init_close() {
        let info = MWF::get_info();
        println!("{}", info);

        assert!(!MWF::is_initialized());

        assert!(MWF::init().is_ok());

        assert!(MWF::is_initialized());

        assert!(MWF::close().is_ok());

        assert!(!MWF::is_initialized());
    }

    #[test]
    fn test_mmf_double_init() {
        assert!(MWF::init().is_ok());

        assert!(matches!(MWF::init(), Err(MWFError::Framework_AlreadyInitialized)));

        assert!(MWF::close().is_ok());
    }

    #[test]
    fn test_mmf_double_close() {
        assert!(matches!(MWF::close(), Err(MWFError::Framework_NotInitialized)));
    }
}
