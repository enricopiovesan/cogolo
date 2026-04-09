//! Native (Rust closure) executor.

use serde_json::Value;

use super::{ArtifactType, CapabilityExecutor, ExecutorCapability, ExecutorError};

/// Handler type alias for native capability implementations.
type NativeHandler = Box<dyn Fn(&Value) -> Result<Value, String> + Send + Sync>;

/// Executes capabilities implemented as native Rust functions.
///
/// The handler is stored as a boxed closure and invoked synchronously.
pub struct NativeExecutor {
    handler: NativeHandler,
}

impl std::fmt::Debug for NativeExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeExecutor").finish_non_exhaustive()
    }
}

impl NativeExecutor {
    /// Create a new [`NativeExecutor`] backed by `handler`.
    pub fn new(handler: impl Fn(&Value) -> Result<Value, String> + Send + Sync + 'static) -> Self {
        Self {
            handler: Box::new(handler),
        }
    }
}

impl CapabilityExecutor for NativeExecutor {
    fn execute(
        &self,
        capability: &ExecutorCapability,
        input: &Value,
    ) -> Result<Value, ExecutorError> {
        if capability.artifact_type != ArtifactType::Native {
            return Err(ExecutorError::UnsupportedArtifactType);
        }
        (self.handler)(input).map_err(ExecutorError::ExecutionFailed)
    }
}
