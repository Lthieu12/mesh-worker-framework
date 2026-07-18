//! Validate MMF framework lifecycle operations.
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
        let info = MMF::get_info();
        println!("{}", info);

        assert!(!MMF::is_initialized());

        assert!(MMF::init().is_ok());

        assert!(MMF::is_initialized());

        assert!(MMF::close().is_ok());

        assert!(!MMF::is_initialized());
    }

    #[test]
    fn test_mmf_double_init() {
        assert!(MMF::init().is_ok());

        assert!(matches!(MMF::init(), Err(MMFError::Framework_AlreadyInitialized)));

        assert!(MMF::close().is_ok());
    }

    #[test]
    fn test_mmf_double_close() {
        assert!(matches!(MMF::close(), Err(MMFError::Framework_NotInitialized)));
    }
}
