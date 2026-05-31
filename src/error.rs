#[derive(Debug)]
pub enum FrameworkError {
    FrameworkNotInitialized,
    FrameworkAlreadyInitialized,
    ModuleInitializationFailed,
}