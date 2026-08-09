//! core.transition-action-status — pure action-item status state machine.
//!
//! Evaluates whether `current_status → requested_status` is allowed under a
//! caller-supplied `transition_config`, optionally requiring the actor to be
//! the owner. Covers open/in_progress/blocked/snoozed/done/cancelled.
#![no_std]
#![no_main]

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

static mut INPUT_BUF: [u8; 8192] = [0; 8192];
static mut OUTPUT_BUF: [u8; 4096] = [0; 4096];

#[unsafe(no_mangle)]
pub extern "C" fn _start() {
    unsafe {
        let mut total = 0usize;
        loop {
            let vec = IoVecMut {
                buffer: INPUT_BUF.as_mut_ptr().add(total),
                length: INPUT_BUF.len() - total,
            };
            let mut n = 0usize;
            if fd_read(0, &vec, 1, &mut n) != 0 || n == 0 {
                break;
            }
            total += n;
            if total >= INPUT_BUF.len() {
                break;
            }
        }
        let out_len = evaluate(&INPUT_BUF[..total], &mut OUTPUT_BUF);
        let out = IoVec {
            buffer: OUTPUT_BUF.as_ptr(),
            length: out_len,
        };
        let mut written = 0usize;
        let _ = fd_write(1, &out, 1, &mut written);
    }
}

unsafe fn evaluate(input: &[u8], out: &mut [u8]) -> usize {
    let current = extract_string_at_depth(input, b"\"current_status\"", 1);
    let requested = extract_string_at_depth(input, b"\"requested_status\"", 1);
    let actor = extract_string_at_depth(input, b"\"actor_id\"", 1);
    let owner = extract_string_at_depth(input, b"\"owner_id\"", 1);
    let config = object_after_key(input, b"\"transition_config\"").unwrap_or(b"");

    if current.is_empty() || requested.is_empty() || actor.is_empty() || config.is_empty() {
        return write_result(
            out,
            false,
            if current.is_empty() {
                b""
            } else {
                current
            },
            b"invalid_status",
            br#"["precondition failed: required fields missing"]"#,
        );
    }

    if !is_known_status(current) || !is_known_status(requested) {
        return write_result(
            out,
            false,
            current,
            b"invalid_status",
            br#"["status value is not a known action-item status"]"#,
        );
    }

    let owner_only = extract_bool(config, b"\"owner_only\"").unwrap_or(true);
    if owner_only {
        if owner.is_empty() || actor != owner {
            let mut trace = [0u8; 96];
            let mut t = 0usize;
            t = copy(&mut trace, t, br#"["owner_only=true and actor is not owner"]"#);
            return write_result(out, false, current, b"not_owner", &trace[..t]);
        }
    }

    let transitions = object_after_key(config, b"\"allowed_transitions\"").unwrap_or(b"");
    if transitions.is_empty() {
        return write_result(
            out,
            false,
            current,
            b"illegal_transition",
            br#"["allowed_transitions missing"]"#,
        );
    }

    // Look up transitions[<current>] as a JSON array and see if <requested> is listed.
    let mut key_buf = [0u8; 64];
    let mut k = 0usize;
    k = copy(&mut key_buf, k, b"\"");
    k = copy(&mut key_buf, k, current);
    k = copy(&mut key_buf, k, b"\"");
    let Some(targets) = array_after_key(transitions, &key_buf[..k]) else {
        let mut trace = [0u8; 128];
        let mut t = 0usize;
        t = copy(&mut trace, t, b"[\"");
        t = copy(&mut trace, t, current);
        t = copy(&mut trace, t, b" has no transition list\"]");
        return write_result(out, false, current, b"illegal_transition", &trace[..t]);
    };

    if !array_contains_string(targets, requested) {
        let mut trace = [0u8; 160];
        let mut t = 0usize;
        t = copy(&mut trace, t, b"[\"");
        t = copy(&mut trace, t, current);
        t = copy(&mut trace, t, b" has no allowed transition to ");
        t = copy(&mut trace, t, requested);
        t = copy(&mut trace, t, b"\"]");
        return write_result(out, false, current, b"illegal_transition", &trace[..t]);
    }

    let mut trace = [0u8; 192];
    let mut t = 0usize;
    t = copy(&mut trace, t, b"[\"");
    t = copy(&mut trace, t, current);
    t = copy(&mut trace, t, " → ".as_bytes());
    t = copy(&mut trace, t, requested);
    t = copy(&mut trace, t, b" is allowed\"");
    if owner_only {
        t = copy(&mut trace, t, b", \"actor is owner\"");
    }
    t = copy(&mut trace, t, b"]");
    write_result(out, true, requested, b"ok", &trace[..t])
}

fn is_known_status(s: &[u8]) -> bool {
    matches!(
        s,
        b"open" | b"in_progress" | b"blocked" | b"snoozed" | b"done" | b"cancelled"
    )
}

fn write_result(
    out: &mut [u8],
    allowed: bool,
    new_status: &[u8],
    reason_code: &[u8],
    evaluation_trace: &[u8],
) -> usize {
    let mut i = 0usize;
    i = copy(out, i, b"{\"allowed\":");
    i = copy(out, i, if allowed { b"true" } else { b"false" });
    i = copy(out, i, b",\"new_status\":\"");
    i = copy(out, i, new_status);
    i = copy(out, i, b"\",\"reason_code\":\"");
    i = copy(out, i, reason_code);
    i = copy(out, i, b"\",\"evaluation_trace\":");
    i = copy(out, i, evaluation_trace);
    i = copy(out, i, b"}");
    i
}

fn object_after_key<'a>(hay: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let pos = find(hay, key)?;
    let after = &hay[pos + key.len()..];
    let colon = after.iter().position(|b| *b == b':')?;
    let mut rest = &after[colon + 1..];
    while rest.first() == Some(&b' ')
        || rest.first() == Some(&b'\n')
        || rest.first() == Some(&b'\t')
    {
        rest = &rest[1..];
    }
    if rest.first() != Some(&b'{') {
        return None;
    }
    let end = balanced_end(rest, b'{', b'}')?;
    Some(&rest[..=end])
}

fn array_after_key<'a>(hay: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let pos = find(hay, key)?;
    let after = &hay[pos + key.len()..];
    let colon = after.iter().position(|b| *b == b':')?;
    let mut rest = &after[colon + 1..];
    while rest.first() == Some(&b' ')
        || rest.first() == Some(&b'\n')
        || rest.first() == Some(&b'\t')
    {
        rest = &rest[1..];
    }
    if rest.first() != Some(&b'[') {
        return None;
    }
    let end = balanced_end(rest, b'[', b']')?;
    Some(&rest[..=end])
}

fn array_contains_string(array: &[u8], value: &[u8]) -> bool {
    let mut needle = [0u8; 80];
    let mut n = 0usize;
    n = copy(&mut needle, n, b"\"");
    n = copy(&mut needle, n, value);
    n = copy(&mut needle, n, b"\"");
    contains(array, &needle[..n])
}

fn balanced_end(s: &[u8], open: u8, close: u8) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_str = false;
    let mut i = 0usize;
    while i < s.len() {
        let b = s[i];
        if in_str {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match b {
            b'"' => in_str = true,
            x if x == open => depth += 1,
            x if x == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn extract_string_at_depth<'a>(hay: &'a [u8], key: &[u8], depth: i32) -> &'a [u8] {
    let Some(pos) = find_key_at_depth(hay, key, depth) else {
        return b"";
    };
    string_value_after(&hay[pos + key.len()..])
}

fn find_key_at_depth(hay: &[u8], key: &[u8], target_depth: i32) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_str = false;
    let mut i = 0usize;
    while i + key.len() <= hay.len() {
        let b = hay[i];
        if in_str {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match b {
            b'"' => {
                if depth == target_depth && hay[i..].starts_with(key) {
                    return Some(i);
                }
                in_str = true;
            }
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    None
}

fn string_value_after<'a>(after_key: &'a [u8]) -> &'a [u8] {
    let Some(colon) = after_key.iter().position(|b| *b == b':') else {
        return b"";
    };
    let mut rest = &after_key[colon + 1..];
    while rest.first() == Some(&b' ') || rest.first() == Some(&b'\n') || rest.first() == Some(&b'\t')
    {
        rest = &rest[1..];
    }
    if rest.first() != Some(&b'"') {
        return b"";
    }
    rest = &rest[1..];
    let Some(end) = rest.iter().position(|b| *b == b'"') else {
        return b"";
    };
    &rest[..end]
}

fn extract_bool(hay: &[u8], key: &[u8]) -> Option<bool> {
    let pos = find(hay, key)?;
    let after = &hay[pos + key.len()..];
    let colon = after.iter().position(|b| *b == b':')?;
    let mut rest = &after[colon + 1..];
    while rest.first() == Some(&b' ') {
        rest = &rest[1..];
    }
    if rest.starts_with(b"true") {
        Some(true)
    } else if rest.starts_with(b"false") {
        Some(false)
    } else {
        None
    }
}

fn copy(out: &mut [u8], at: usize, bytes: &[u8]) -> usize {
    let end = at + bytes.len();
    if end > out.len() {
        return at;
    }
    out[at..end].copy_from_slice(bytes);
    end
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
