//! Capability executor abstraction for Traverse.
//!
//! Governed by spec `025-wasm-executor-adapter`.
//!
//! Two concrete implementations:
//! - [`NativeExecutor`] — executes capabilities implemented as native Rust closures.
//! - [`WasmExecutor`] — executes capabilities compiled to `wasm32-wasi` binaries via Wasmtime.

pub mod native;
pub mod wasm;

pub use native::NativeExecutor;
pub use wasm::WasmExecutor;

use serde_json::Value;

/// The artifact type recorded in a capability registration, used to route to the correct executor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactType {
    /// Native Rust implementation — executed via [`NativeExecutor`].
    Native,
    /// WASM binary — executed via [`WasmExecutor`].
    Wasm,
}

/// A resolved capability ready for execution.
#[derive(Debug, Clone)]
pub struct ExecutorCapability {
    /// Unique capability identifier.
    pub capability_id: String,
    /// How the binary is packaged.
    pub artifact_type: ArtifactType,
    /// File-system path to the `.wasm` binary (only relevant for `ArtifactType::Wasm`).
    pub wasm_binary_path: Option<String>,
    /// Expected SHA-256 hex digest of the WASM binary (only relevant for `ArtifactType::Wasm`).
    pub wasm_checksum: Option<String>,
}

/// Error returned by a [`CapabilityExecutor`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorError {
    /// The WASM binary could not be loaded from the given path.
    BinaryLoadFailed(String),
    /// The SHA-256 checksum of the loaded binary did not match the expected value.
    ChecksumMismatch { expected: String, actual: String },
    /// The Wasmtime engine or linker could not be configured.
    RuntimeSetupFailed(String),
    /// The WASM module trapped or returned a non-zero exit code.
    ExecutionFailed(String),
    /// The executor produced output that could not be parsed as JSON.
    OutputDeserializationFailed(String),
    /// The executor type does not support the requested capability.
    UnsupportedArtifactType,
}

impl std::fmt::Display for ExecutorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BinaryLoadFailed(msg) => write!(f, "binary load failed: {msg}"),
            Self::ChecksumMismatch { expected, actual } => {
                write!(f, "checksum mismatch: expected {expected}, got {actual}")
            }
            Self::RuntimeSetupFailed(msg) => write!(f, "runtime setup failed: {msg}"),
            Self::ExecutionFailed(msg) => write!(f, "execution failed: {msg}"),
            Self::OutputDeserializationFailed(msg) => {
                write!(f, "output deserialization failed: {msg}")
            }
            Self::UnsupportedArtifactType => write!(f, "unsupported artifact type for this executor"),
        }
    }
}

impl std::error::Error for ExecutorError {}

/// Trait implemented by all capability executors.
///
/// Executors are stateless; all context is passed per call.
pub trait CapabilityExecutor: Send + Sync {
    /// Execute `capability` with `input`, returning the output or an error.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutorError`] when execution cannot be completed.
    fn execute(
        &self,
        capability: &ExecutorCapability,
        input: &Value,
    ) -> Result<Value, ExecutorError>;
}
