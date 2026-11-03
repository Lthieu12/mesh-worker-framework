use std::any::Any;

use crate::MWFError;

/// Opaque container transporting application-defined data.
///
/// `MessageData` can store any Rust type implementing
/// `Clone + Send + Sync + 'static`.
///
/// The contained value can later be recovered using
/// `MessageData::into()`.
pub struct MessageData {
    /// Stored message data.
    value: Box<dyn Any + Send + Sync>,

    /// Fully qualified Rust type name of the stored message data.
    type_name: &'static str,
}

impl MessageData {
    /// Creates a new MessageData by cloning the provided value.
    pub fn new<T>(value: &T) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        Self { value: Box::new(value.clone()), type_name: std::any::type_name::<T>() }
    }

    /// Returns true if the stored message data is of type `T`.
    pub fn is<T>(&self) -> bool
    where
        T: Send + Sync + 'static,
    {
        self.value.is::<T>()
    }

    /// Returns the fully qualified Rust type name of the stored message data.
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// Consumes this MessageData and returns the stored value as type `T`.
    pub fn into<T>(self) -> Result<T, MWFError>
    where
        T: Send + Sync + 'static,
    {
        // Retrieve the expected type name.
        let expected_type = std::any::type_name::<T>();

        // Downcast the stored value to the requested type.
        let value = self.value.downcast::<T>().map_err(|_| MWFError::Message_DataTypeMismatch { expected_type, actual_type: self.type_name })?;

        Ok(*value)
    }
}

impl std::fmt::Debug for MessageData {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("MessageData").field("type_name", &self.type_name).finish()
    }
}
