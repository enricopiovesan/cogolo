//! Audited, bounded WASI Preview 1 stdin/stdout/status boundary.
//!
//! Governed by Spec 090 and ADR-0031. Keep all unsafe code and direct WASI
//! imports in this file. It deliberately imports only fd_read, fd_write, and
//! proc_exit; it must never become a general guest host API.

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

const STDIN: u32 = 0;
const STDOUT: u32 = 1;
const MAX_INPUT_BYTES: usize = 64 * 1024;

#[repr(C)]
struct Iovec {
    buf: *mut u8,
    buf_len: usize,
}

#[link(wasm_import_module = "wasi_snapshot_preview1")]
unsafe extern "C" {
    fn fd_read(fd: u32, iovs: *const Iovec, iovs_len: usize, nread: *mut usize) -> u16;
    fn fd_write(fd: u32, iovs: *const Iovec, iovs_len: usize, nwritten: *mut usize) -> u16;
    fn proc_exit(status: u32) -> !;
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn memcmp(left: *const u8, right: *const u8, count: usize) -> i32 {
    for index in 0..count {
        // SAFETY: callers follow the C memcmp contract and provide readable
        // ranges of `count` bytes; this symbol exists solely to satisfy the
        // guest-local JSON runtime, and does not access host memory.
        let left_byte = unsafe { *left.add(index) };
        let right_byte = unsafe { *right.add(index) };
        if left_byte != right_byte {
            return i32::from(left_byte) - i32::from(right_byte);
        }
    }
    0
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn _start() {
    if crate::run().is_err() {
        exit_failure();
    }
}

#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    exit_failure()
}

pub fn read_stdin() -> Result<String, String> {
    let mut bytes = Vec::with_capacity(4096);
    loop {
        if bytes.len() == MAX_INPUT_BYTES {
            return Err("stdin request exceeds 65536-byte limit".to_string());
        }
        let mut chunk = [0_u8; 4096];
        let iovec = Iovec {
            buf: chunk.as_mut_ptr(),
            buf_len: chunk.len(),
        };
        let mut read = 0_usize;
        // SAFETY: the iovec and count pointers point to valid writable stack
        // memory for this synchronous WASI call; no pointer escapes the call.
        let errno = unsafe { fd_read(STDIN, &iovec, 1, &mut read) };
        if errno != 0 {
            return Err(format!("WASI fd_read failed with errno {errno}"));
        }
        if read > chunk.len() {
            return Err("WASI fd_read returned an invalid byte count".to_string());
        }
        if read == 0 {
            break;
        }
        let remaining = MAX_INPUT_BYTES - bytes.len();
        if read > remaining {
            return Err("stdin request exceeds 65536-byte limit".to_string());
        }
        bytes.extend_from_slice(&chunk[..read]);
    }
    String::from_utf8(bytes).map_err(|_| "stdin request is not UTF-8".to_string())
}

pub fn write_stdout(bytes: &[u8]) -> Result<(), String> {
    let mut offset = 0_usize;
    while offset < bytes.len() {
        let remaining = &bytes[offset..];
        let iovec = Iovec {
            buf: remaining.as_ptr().cast_mut(),
            buf_len: remaining.len(),
        };
        let mut written = 0_usize;
        // SAFETY: fd_write observes the immutable output buffer only for the
        // synchronous call; the count pointer is valid writable stack memory.
        let errno = unsafe { fd_write(STDOUT, &iovec, 1, &mut written) };
        if errno != 0 {
            return Err(format!("WASI fd_write failed with errno {errno}"));
        }
        if written == 0 || written > remaining.len() {
            return Err("WASI fd_write returned an invalid byte count".to_string());
        }
        offset += written;
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub fn exit_failure() -> ! {
    // SAFETY: proc_exit is the approved terminal status import and never
    // returns to Rust.
    unsafe { proc_exit(1) }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn exit_failure() -> ! {
    std::process::exit(1)
}
