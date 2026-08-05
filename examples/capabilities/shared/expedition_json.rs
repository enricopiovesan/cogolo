// Hand-rolled, no-alloc JSON helpers shared by the expedition-planning WASM
// capabilities (`#![no_std]`, no heap, no serde_json). Included textually via
// `include!` into each capability's `agent.rs` -- there is no cargo workspace
// linking these `rustc`-compiled guest binaries together.
//
// Assumes compact, well-formed JSON with no embedded escaped quotes inside
// string values other than a plain `\"` pass-through, which is true for every
// input Traverse's own runtime sends (canonical `serde_json::to_string`
// output) and every output these capabilities produce.

/// A fixed-capacity output buffer that bytes are appended to as JSON is built
/// field by field.
pub struct Buf<'a> {
    data: &'a mut [u8],
    len: usize,
}

impl<'a> Buf<'a> {
    pub fn new(data: &'a mut [u8]) -> Self {
        Buf { data, len: 0 }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn push_bytes(&mut self, bytes: &[u8]) {
        let n = bytes.len();
        self.data[self.len..self.len + n].copy_from_slice(bytes);
        self.len += n;
    }

    pub fn push_str(&mut self, s: &str) {
        self.push_bytes(s.as_bytes());
    }

    /// Consumes the buffer and returns the written slice.
    pub fn finish(self) -> &'a [u8] {
        &self.data[..self.len]
    }

    /// Writes `raw` (already-unescaped JSON string content bytes, no
    /// surrounding quotes) as a quoted JSON string.
    pub fn push_json_string(&mut self, raw: &[u8]) {
        self.push_bytes(b"\"");
        self.push_bytes(raw);
        self.push_bytes(b"\"");
    }
}

fn skip_ws(json: &[u8], mut i: usize) -> usize {
    while i < json.len() && matches!(json[i], b' ' | b'\n' | b'\t' | b'\r') {
        i += 1;
    }
    i
}

/// Finds `"key"` as an object key anywhere in `json` and returns the byte
/// offset of the start of its value (immediately after the following `:`).
fn find_key_value_start(json: &[u8], key: &str) -> Option<usize> {
    let pattern = key.as_bytes();
    let mut i = 0;
    while i + 1 < json.len() {
        if json[i] == b'"' && json[i + 1..].starts_with(pattern) {
            let after_key = i + 1 + pattern.len();
            if json.get(after_key) == Some(&b'"') {
                let j = skip_ws(json, after_key + 1);
                if json.get(j) == Some(&b':') {
                    return Some(skip_ws(json, j + 1));
                }
            }
        }
        i += 1;
    }
    None
}

/// Parses a JSON string value starting at `start` (must point at the opening
/// `"`). Returns the raw content bytes (unescaped depth: `\"` is skipped as a
/// two-byte unit, not decoded) and the index just past the closing `"`.
fn parse_string_value(json: &[u8], start: usize) -> Option<(&[u8], usize)> {
    if json.get(start) != Some(&b'"') {
        return None;
    }
    let mut i = start + 1;
    while i < json.len() {
        match json[i] {
            b'\\' => i += 2,
            b'"' => return Some((&json[start + 1..i], i + 1)),
            _ => i += 1,
        }
    }
    None
}

/// Byte range `(start, end)` of a JSON object or array value starting at
/// `start`, with balanced bracket matching that correctly skips over quoted
/// strings (so a `{`/`}`/`[`/`]` inside a string value doesn't unbalance it).
fn parse_bracketed_value(json: &[u8], start: usize, open: u8, close: u8) -> Option<(usize, usize)> {
    if json.get(start) != Some(&open) {
        return None;
    }
    let mut depth = 0i32;
    let mut i = start;
    let mut in_string = false;
    while i < json.len() {
        let c = json[i];
        if in_string {
            match c {
                b'\\' => i += 1,
                b'"' => in_string = false,
                _ => {}
            }
        } else if c == b'"' {
            in_string = true;
        } else if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return Some((start, i + 1));
            }
        }
        i += 1;
    }
    None
}

/// Finds the raw content bytes of a top-level-searched string field `"key"`.
pub fn find_string<'a>(json: &'a [u8], key: &str) -> Option<&'a [u8]> {
    let start = find_key_value_start(json, key)?;
    parse_string_value(json, start).map(|(content, _)| content)
}

/// Finds a boolean field `"key"`.
pub fn find_bool(json: &[u8], key: &str) -> Option<bool> {
    let start = find_key_value_start(json, key)?;
    if json[start..].starts_with(b"true") {
        Some(true)
    } else if json[start..].starts_with(b"false") {
        Some(false)
    } else {
        None
    }
}

/// Finds the full `{...}` slice of an object field `"key"`.
pub fn find_object<'a>(json: &'a [u8], key: &str) -> Option<&'a [u8]> {
    let start = find_key_value_start(json, key)?;
    let (s, e) = parse_bracketed_value(json, start, b'{', b'}')?;
    Some(&json[s..e])
}

/// Finds the full `[...]` slice of an array field `"key"`.
pub fn find_array<'a>(json: &'a [u8], key: &str) -> Option<&'a [u8]> {
    let start = find_key_value_start(json, key)?;
    let (s, e) = parse_bracketed_value(json, start, b'[', b']')?;
    Some(&json[s..e])
}

/// Calls `f` with the raw content bytes of every string element of a JSON
/// string array (`arr` is the full `[...]` slice, e.g. from [`find_array`]).
pub fn for_each_string_in_array<F: FnMut(&[u8])>(arr: &[u8], mut f: F) {
    let mut i = 1; // past leading '['
    loop {
        i = skip_ws(arr, i);
        while i < arr.len() && arr[i] == b',' {
            i = skip_ws(arr, i + 1);
        }
        if i >= arr.len() || arr[i] == b']' {
            break;
        }
        match parse_string_value(arr, i) {
            Some((content, end)) => {
                f(content);
                i = end;
            }
            None => break,
        }
    }
}

/// The first string element of a JSON string array, if any.
pub fn first_string_in_array(arr: &[u8]) -> Option<&[u8]> {
    let i = skip_ws(arr, 1);
    if arr.get(i) == Some(&b'"') {
        parse_string_value(arr, i).map(|(content, _)| content)
    } else {
        None
    }
}

/// Writes a `[...]` JSON array of strings to `out` by concatenating the
/// items of every array in `arrays` (each element of `arrays` must be a raw
/// `[...]` slice, e.g. from [`find_array`]), in order, comma-joined.
pub fn push_joined_string_arrays(out: &mut Buf<'_>, arrays: &[&[u8]]) {
    out.push_str("[");
    let mut first = true;
    for arr in arrays {
        for_each_string_in_array(arr, |item| {
            if !first {
                out.push_str(",");
            }
            out.push_json_string(item);
            first = false;
        });
    }
    out.push_str("]");
}

/// ASCII-alphanumeric-only lowercase slug, matching `traverse-cli`'s own
/// `slug()` (`crates/traverse-cli/src/main.rs`): every non-alphanumeric byte
/// is dropped, not replaced.
pub fn write_slug(buf: &mut Buf<'_>, value: &[u8]) {
    let mut any = false;
    for &b in value {
        if b.is_ascii_alphanumeric() {
            buf.push_bytes(&[b.to_ascii_lowercase()]);
            any = true;
        }
    }
    if !any {
        buf.push_str("expedition");
    }
}

// --- Shared WASI stdin/stdout plumbing -------------------------------------
//
// Every capability that includes this file must also define:
//   fn process(input: &[u8], out: &mut Buf<'_>);
// which reads `input` (the real, dynamic request JSON) and writes its result
// into `out`. This module supplies the `_start` entrypoint and the fd_read/
// fd_write host imports (the only two WASI functions this ABI whitelist
// allows, per host_abi_v1.json).

#[repr(C)]
struct IoVec {
    buffer: *const u8,
    length: usize,
}

#[repr(C)]
struct IoVecMut {
    buffer: *mut u8,
    length: usize,
}

#[link(wasm_import_module = "wasi_snapshot_preview1")]
unsafe extern "C" {
    fn fd_read(fd: u32, vectors: *const IoVecMut, count: usize, read: *mut usize) -> u32;
    fn fd_write(fd: u32, vectors: *const IoVec, count: usize, written: *mut usize) -> u32;
}

const INPUT_CAPACITY: usize = 8192;
const OUTPUT_CAPACITY: usize = 8192;

static mut INPUT_BUF: [u8; INPUT_CAPACITY] = [0; INPUT_CAPACITY];
static mut OUTPUT_BUF: [u8; OUTPUT_CAPACITY] = [0; OUTPUT_CAPACITY];

#[unsafe(no_mangle)]
pub extern "C" fn _start() {
    unsafe {
        let input_ptr = &raw mut INPUT_BUF;
        let mut total = 0usize;
        loop {
            let remaining = INPUT_CAPACITY - total;
            if remaining == 0 {
                break;
            }
            let vec = IoVecMut {
                buffer: (*input_ptr).as_mut_ptr().add(total),
                length: remaining,
            };
            let mut n = 0usize;
            if fd_read(0, &vec, 1, &mut n) != 0 || n == 0 {
                break;
            }
            total += n;
        }
        let input: &[u8] = &(&*input_ptr)[..total];

        let output_ptr = &raw mut OUTPUT_BUF;
        let mut out = Buf::new(&mut *output_ptr);
        process(input, &mut out);
        let out_len = out.len();

        let out_vec = IoVec {
            buffer: (*output_ptr).as_ptr(),
            length: out_len,
        };
        let mut written = 0usize;
        let _ = fd_write(1, &out_vec, 1, &mut written);
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
