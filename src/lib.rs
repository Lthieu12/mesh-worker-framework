pub mod error;
pub mod module;
pub mod message;
pub mod registry;
pub mod mmf;
pub mod logging;

#[cfg(test)]
mod tests {

    use crate::error::FrameworkError;
    use crate::mmf::MMF;

    #[test]
    fn test_mmf_init_close() {
        assert!(!MMF::is_initialized());
        assert!(MMF::init().is_ok());
        assert!(MMF::is_initialized());
        assert!(MMF::close().is_ok());
        assert!(!MMF::is_initialized());
    }

    #[test]
    fn test_mmf_double_init() {
        assert!(MMF::init().is_ok());
        assert!(
            matches!(
                MMF::init(),
                Err(FrameworkError::FrameworkAlreadyInitialized)
            )
        );
        assert!(MMF::close().is_ok());
    }

    #[test]
    fn test_mmf_double_close() {
        assert!(
            matches!(
                MMF::close(),
                Err(FrameworkError::FrameworkNotInitialized)
            )
        );
    }
}