use serde_json::json;
use traverse_runtime::executor::{
    ArtifactType, CapabilityExecutor, ExecutorCapability, ExecutorError, NativeExecutor,
    WasmExecutor,
};

// --- NativeExecutor tests ---

#[test]
fn native_executor_runs_handler() {
    let executor = NativeExecutor::new(|input| {
        let name = input["name"].as_str().unwrap_or("world");
        Ok(json!({ "greeting": format!("hello, {name}!") }))
    });

    let cap = native_capability("greet");
    let result = executor.execute(&cap, &json!({ "name": "traverse" }));

    assert_eq!(result, Ok(json!({ "greeting": "hello, traverse!" })));
}

#[test]
fn native_executor_propagates_handler_error() {
    let executor = NativeExecutor::new(|_| Err("something went wrong".to_string()));

    let cap = native_capability("fail");
    let err = executor.execute(&cap, &json!({})).unwrap_err();

    assert_eq!(err, ExecutorError::ExecutionFailed("something went wrong".to_string()));
}

#[test]
fn native_executor_rejects_wasm_artifact_type() {
    let executor = NativeExecutor::new(|_| Ok(json!({})));

    let cap = ExecutorCapability {
        capability_id: "wrong-type".to_string(),
        artifact_type: ArtifactType::Wasm,
        wasm_binary_path: None,
        wasm_checksum: None,
    };
    let err = executor.execute(&cap, &json!({})).unwrap_err();

    assert_eq!(err, ExecutorError::UnsupportedArtifactType);
}

#[test]
fn native_executor_passes_input_through() {
    let executor = NativeExecutor::new(|input| Ok(input.clone()));

    let cap = native_capability("echo");
    let input = json!({ "a": 1, "b": [true, false] });
    let result = executor.execute(&cap, &input).unwrap();

    assert_eq!(result, input);
}

// --- WasmExecutor tests ---

#[test]
fn wasm_executor_rejects_native_artifact_type() {
    let executor = WasmExecutor::new().expect("engine init");

    let cap = native_capability("wrong");
    let err = executor.execute(&cap, &json!({})).unwrap_err();

    assert_eq!(err, ExecutorError::UnsupportedArtifactType);
}

#[test]
fn wasm_executor_errors_when_no_path_set() {
    let executor = WasmExecutor::new().expect("engine init");

    let cap = ExecutorCapability {
        capability_id: "no-path".to_string(),
        artifact_type: ArtifactType::Wasm,
        wasm_binary_path: None,
        wasm_checksum: None,
    };
    let err = executor.execute(&cap, &json!({})).unwrap_err();

    assert!(
        matches!(err, ExecutorError::BinaryLoadFailed(_)),
        "expected BinaryLoadFailed, got {err:?}"
    );
}

#[test]
fn wasm_executor_errors_on_missing_file() {
    let executor = WasmExecutor::new().expect("engine init");

    let cap = ExecutorCapability {
        capability_id: "missing".to_string(),
        artifact_type: ArtifactType::Wasm,
        wasm_binary_path: Some("/nonexistent/path/module.wasm".to_string()),
        wasm_checksum: None,
    };
    let err = executor.execute(&cap, &json!({})).unwrap_err();

    assert!(
        matches!(err, ExecutorError::BinaryLoadFailed(_)),
        "expected BinaryLoadFailed, got {err:?}"
    );
}

#[test]
fn wasm_executor_detects_checksum_mismatch() {
    let executor = WasmExecutor::new().expect("engine init");

    // Build a minimal WAT module that just returns immediately
    let wat_src = r#"
        (module
            (memory 1)
            (func $main (export "_start"))
        )
    "#;
    let wasm_bytes = wat::parse_str(wat_src).expect("valid WAT");

    // Write to a temp file
    let tmp = tempfile_path();
    std::fs::write(&tmp, &wasm_bytes).expect("write temp wasm");

    let cap = ExecutorCapability {
        capability_id: "checksum-test".to_string(),
        artifact_type: ArtifactType::Wasm,
        wasm_binary_path: Some(tmp.clone()),
        wasm_checksum: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
    };

    let err = executor.execute(&cap, &json!({})).unwrap_err();
    std::fs::remove_file(&tmp).ok();

    assert!(
        matches!(err, ExecutorError::ChecksumMismatch { .. }),
        "expected ChecksumMismatch, got {err:?}"
    );
}

#[test]
fn wasm_executor_runs_echo_module() {
    let executor = WasmExecutor::new().expect("engine init");

    // WAT module that reads stdin and writes it back to stdout (echo)
    // Uses WASI fd_read (fd=0) and fd_write (fd=1)
    let wat_src = r#"
        (module
            (import "wasi_snapshot_preview1" "fd_read"
                (func $fd_read (param i32 i32 i32 i32) (result i32)))
            (import "wasi_snapshot_preview1" "fd_write"
                (func $fd_write (param i32 i32 i32 i32) (result i32)))
            (import "wasi_snapshot_preview1" "proc_exit"
                (func $proc_exit (param i32)))
            (memory (export "memory") 1)
            (func $_start (export "_start")
                ;; iovec for read: ptr=8, len=4096
                (i32.store (i32.const 0) (i32.const 8))
                (i32.store (i32.const 4) (i32.const 4096))
                ;; read stdin into offset 8
                (drop (call $fd_read (i32.const 0) (i32.const 0) (i32.const 1) (i32.const 4100)))
                ;; nread is at memory[4100]; use it as iovec len for write
                (i32.store (i32.const 0) (i32.const 8))
                (i32.store (i32.const 4) (i32.load (i32.const 4100)))
                ;; write stdout
                (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 4104)))
            )
        )
    "#;

    let wasm_bytes = wat::parse_str(wat_src).expect("valid WAT");
    let input = json!({ "key": "value" });

    let result = executor.run_bytes(&wasm_bytes, &input);
    assert_eq!(result, Ok(input), "echo module should return input unchanged");
}

#[test]
fn wasm_executor_rejects_invalid_json_output() {
    let executor = WasmExecutor::new().expect("engine init");

    // WAT module that writes "not-json" to stdout
    let wat_src = r#"
        (module
            (import "wasi_snapshot_preview1" "fd_write"
                (func $fd_write (param i32 i32 i32 i32) (result i32)))
            (memory (export "memory") 1)
            (data (i32.const 16) "not-json")
            (func $_start (export "_start")
                ;; iovec: ptr=16, len=8
                (i32.store (i32.const 0) (i32.const 16))
                (i32.store (i32.const 4) (i32.const 8))
                (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 8)))
            )
        )
    "#;

    let wasm_bytes = wat::parse_str(wat_src).expect("valid WAT");
    let err = executor.run_bytes(&wasm_bytes, &json!({})).unwrap_err();

    assert!(
        matches!(err, ExecutorError::OutputDeserializationFailed(_)),
        "expected OutputDeserializationFailed, got {err:?}"
    );
}

// --- helpers ---

fn native_capability(id: &str) -> ExecutorCapability {
    ExecutorCapability {
        capability_id: id.to_string(),
        artifact_type: ArtifactType::Native,
        wasm_binary_path: None,
        wasm_checksum: None,
    }
}

fn tempfile_path() -> String {
    format!("/tmp/traverse-test-{}.wasm", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0))
}
