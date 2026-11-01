use super::super::MWF;
use super::super::MWF_DATA;
use crate::framework_error::MWFError;
use crate::framework_error::MWFPanic;
use crate::worker::Worker;
use crate::worker::WorkerConstructor;

impl MWF {
    /// Register a worker type.
    ///
    /// worker types must be registered before they can be
    /// instantiated by MWF.
    ///
    /// Registration associates the worker's WorkerTypeID
    /// with:
    /// - its Rust type name, used for diagnostics;
    /// - a constructor function used by MWF to create new
    ///   worker instances dynamically.
    ///
    /// The registered constructor creates a new default
    /// instance of the worker each time MWF spawns it.
    ///
    /// Returns:
    /// - Framework_NotInitialized if MWF has not been initialized.
    /// - WorkerType_AlreadyRegistered if the worker type has
    ///   already been registered.
    /// - Worker_DuplicateWorkerTypeID if the WorkerTypeID is
    ///   already associated with a different Rust worker type.
    pub fn register_worker_type<TWorker>() -> Result<(), MWFError>
    where
        TWorker: Worker + Send + 'static,
    {
        // Ensure MWF is initialized and the current worker is still registered, when called from a worker context.
        drop(MWF::validate_api_context()?);

        let mut mwf_data_guard = MWF_DATA.write().expect(MWFPanic::MWFData_WriteLockPoisoned.message());
        let mwf_data = mwf_data_guard.as_mut().ok_or(MWFError::Framework_NotInitialized)?;

        let worker_type_id = TWorker::worker_type_id();
        if !worker_type_id.is_valid() {
            return Err(MWFError::WorkerType_Invalid { worker_type_id });
        }

        let rust_type_name = std::any::type_name::<TWorker>();

        match mwf_data.worker_type_registry.get_worker_rust_type_name(&TWorker::worker_type_id()) {
            Ok(registered_rust_type_name) => {
                if registered_rust_type_name == rust_type_name {
                    return Err(MWFError::WorkerType_AlreadyRegistered);
                }

                return Err(MWFError::WorkerType_DuplicateWorkerTypeID);
            }

            Err(MWFError::WorkerType_NotFound) => {
                // Create the constructor associated with this worker type.
                //
                // The constructor is stored in the WorkerTypeRegistry and
                // invoked by MWF whenever a new instance of this worker
                // type must be spawned.
                //
                // Note that T::new() is not executed here.
                // Instead, a function calling T::new() is created and
                // stored for later use.
                let worker_constructor: WorkerConstructor = || Box::new(TWorker::new());
                mwf_data.worker_type_registry.add_worker_type(&TWorker::worker_type_id(), rust_type_name, worker_constructor);
            }

            Err(error) => return Err(error),
        }

        Ok(())
    }
}
