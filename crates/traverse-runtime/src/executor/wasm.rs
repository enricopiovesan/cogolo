//! Wasmtime-backed WASM executor.
//!
//! Executes `wasm32-wasi` capability binaries inside a sandboxed Wasmtime engine.
//! Input is fed via WASI stdin; output is captured from WASI stdout.
//! No ambient WASI authority is granted — all capabilities are deny-by-default.

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::WasiCtxBuilder;
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};

use super::{ArtifactType, CapabilityExecutor, ExecutorCapability, ExecutorError};

/// Executes `.wasm32-wasi` capability binaries via Wasmtime.
///
/// Every invocation creates a fresh Wasmtime `Store` — no state leaks between calls.
#[derive(Debug)]
pub struct WasmExecutor {
    engine: Engine,
}

impl WasmExecutor {
    /// Create a new [`WasmExecutor`] with a default Wasmtime engine.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutorError::RuntimeSetupFailed`] if Wasmtime cannot initialise.
    pub fn new() -> Result<Self, ExecutorError> {
        let engine = Engine::default();
        Ok(Self { engine })
    }
}

impl CapabilityExecutor for WasmExecutor {
    fn execute(
        &self,
        capability: &ExecutorCapability,
        input: &Value,
    ) -> Result<Value, ExecutorError> {
        if capability.artifact_type != ArtifactType::Wasm {
            return Err(ExecutorError::UnsupportedArtifactType);
        }

        // --- Load binary ---
        let wasm_path = capability.wasm_binary_path.as_deref().ok_or_else(|| {
            ExecutorError::BinaryLoadFailed("no wasm_binary_path set".to_string())
        })?;

        let binary = fs::read(wasm_path).map_err(|e| {
            ExecutorError::BinaryLoadFailed(format!("cannot read {wasm_path}: {e}"))
        })?;

        // --- Checksum validation ---
        if let Some(expected) = capability.wasm_checksum.as_deref() {
            let actual = sha256_hex(&binary);
            if actual != expected {
                return Err(ExecutorError::ChecksumMismatch {
                    expected: expected.to_string(),
                    actual,
                });
            }
        }

        self.run_wasm(&binary, input)
    }
}

impl WasmExecutor {
    /// Execute pre-loaded WASM bytes with the given input.
    ///
    /// Exposed separately so tests can pass raw bytes without needing a file on disk.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutorError`] if input serialization fails, the WASM module cannot be
    /// compiled or linked, execution fails, or stdout is not valid JSON.
    pub fn run_bytes(&self, wasm_bytes: &[u8], input: &Value) -> Result<Value, ExecutorError> {
        self.run_wasm(wasm_bytes, input)
    }

    fn run_wasm(&self, wasm_bytes: &[u8], input: &Value) -> Result<Value, ExecutorError> {
        let input_json = serde_json::to_string(input)
            .map_err(|e| ExecutorError::ExecutionFailed(format!("input serialization: {e}")))?;

        // Clone pipe reference before passing to builder — needed to read output after execution
        let stdout_pipe = MemoryOutputPipe::new(65536);
        let stdout_ref = stdout_pipe.clone();

        // Build a WASI context: stdin = input JSON, stdout = captured buffer
        // No filesystem, no network, no env vars — deny-by-default
        let wasi_ctx: WasiP1Ctx = WasiCtxBuilder::new()
            .stdin(MemoryInputPipe::new(input_json.into_bytes()))
            .stdout(stdout_pipe)
            .build_p1();

        let mut linker: Linker<WasiP1Ctx> = Linker::new(&self.engine);
        wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |s| s)
            .map_err(|e| ExecutorError::RuntimeSetupFailed(e.to_string()))?;

        let mut store: Store<WasiP1Ctx> = Store::new(&self.engine, wasi_ctx);

        let module = Module::from_binary(&self.engine, wasm_bytes)
            .map_err(|e| ExecutorError::RuntimeSetupFailed(format!("module compile: {e}")))?;

        linker
            .module(&mut store, "", &module)
            .map_err(|e| ExecutorError::RuntimeSetupFailed(format!("module link: {e}")))?;

        linker
            .get_default(&mut store, "")
            .map_err(|e| ExecutorError::RuntimeSetupFailed(format!("get_default: {e}")))?
            .typed::<(), ()>(&store)
            .map_err(|e| ExecutorError::RuntimeSetupFailed(format!("typed: {e}")))?
            .call(&mut store, ())
            .map_err(|e| ExecutorError::ExecutionFailed(e.to_string()))?;

        // Extract captured stdout — contents() reads the buffer without consuming it
        let raw_output = stdout_ref.contents();

        serde_json::from_slice::<Value>(&raw_output).map_err(|e| {
            ExecutorError::OutputDeserializationFailed(format!(
                "stdout is not valid JSON: {e} — raw: {}",
                String::from_utf8_lossy(&raw_output)
            ))
        })
    }
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}
