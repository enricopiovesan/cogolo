//! core.validate-action-item — pure action-item commitment gatekeeper.
//!
//! Validates title/owner/due-date rules and optional duplicate detection
//! against existing open items under a caller-supplied validation_config.
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

static mut INPUT_BUF: [u8; 12288] = [0; 12288];
static mut OUTPUT_BUF: [u8; 8192] = [0; 8192];

const MAX_ERRORS: usize = 4;
const MAX_TRACE: usize = 6;

#[derive(Clone, Copy)]
struct ErrPart {
    field: &'static [u8],
    code: &'static [u8],
    message: &'static [u8],
}

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
    let item = object_after_key(input, b"\"action_item\"").unwrap_or(b"");
    let config = object_after_key(input, b"\"validation_config\"").unwrap_or(b"");
    let existing = array_after_key(input, b"\"existing_open_items\"").unwrap_or(b"[]");
    let reference_date = extract_string_at_depth(input, b"\"reference_date\"", 1);

    if item.is_empty() || config.is_empty() || reference_date.is_empty() {
        return write_invalid_config(out, br#"["precondition failed: required fields missing"]"#);
    }

    let title = extract_string(item, b"\"title\"");
    let owner = extract_string(item, b"\"owner_id\"");
    let due = extract_string(item, b"\"due_date\"");

    let require_owner = extract_bool(config, b"\"require_owner\"").unwrap_or(true);
    let require_due = extract_bool(config, b"\"require_due_date\"").unwrap_or(false);
    let allow_past = extract_bool(config, b"\"allow_past_due\"").unwrap_or(false);
    let dup_mode = extract_string(config, b"\"duplicate_check\"");
    let dup_mode = if dup_mode.is_empty() {
        b"title_and_owner"
    } else {
        dup_mode
    };

    if !matches!(dup_mode, b"none" | b"title" | b"title_and_owner") {
        return write_invalid_config(out, br#"["invalid_config: duplicate_check"]"#);
    }

    let mut errors = [ErrPart {
        field: b"",
        code: b"",
        message: b"",
    }; MAX_ERRORS];
    let mut err_n = 0usize;
    let mut traces: [&[u8]; MAX_TRACE] = [b""; MAX_TRACE];
    let mut trace_n = 0usize;

    if title.is_empty() {
        err_n = push_err(
            &mut errors,
            err_n,
            b"title",
            b"empty_title",
            b"title must be non-empty",
        );
        trace_n = push_trace(&mut traces, trace_n, b"title missing or empty");
    }

    if require_owner && owner.is_empty() {
        err_n = push_err(
            &mut errors,
            err_n,
            b"owner_id",
            b"missing_owner",
            b"owner_id is required",
        );
        trace_n = push_trace(
            &mut traces,
            trace_n,
            b"require_owner=true and owner_id missing",
        );
    }

    if require_due && due.is_empty() {
        err_n = push_err(
            &mut errors,
            err_n,
            b"due_date",
            b"missing_due_date",
            b"due_date is required",
        );
        trace_n = push_trace(
            &mut traces,
            trace_n,
            b"require_due_date=true and due_date missing",
        );
    }

    if !due.is_empty() && !allow_past && due < reference_date {
        err_n = push_err(
            &mut errors,
            err_n,
            b"due_date",
            b"past_due",
            b"due_date is in the past",
        );
        trace_n = push_trace(
            &mut traces,
            trace_n,
            b"due_date is earlier than reference_date",
        );
    }

    let mut is_duplicate = false;
    if err_n == 0 && dup_mode != b"none" && !title.is_empty() {
        if find_duplicate(existing, title, owner, dup_mode == b"title_and_owner") {
            is_duplicate = true;
            err_n = push_err(
                &mut errors,
                err_n,
                b"title",
                b"duplicate",
                b"matching open item already exists",
            );
            let t: &[u8] = if dup_mode == b"title_and_owner" {
                b"duplicate_check=title_and_owner matched existing open item"
            } else {
                b"duplicate_check=title matched existing open item"
            };
            trace_n = push_trace(&mut traces, trace_n, t);
        }
    }

    if err_n == 0 {
        trace_n = push_trace(&mut traces, trace_n, b"all checks passed");
        return write_ok(out, title, owner, due, &traces[..trace_n]);
    }

    let reason: &[u8] = if is_duplicate {
        b"duplicate"
    } else {
        b"validation_failed"
    };
    write_fail(out, reason, &errors[..err_n], &traces[..trace_n])
}

fn push_err(
    errors: &mut [ErrPart; MAX_ERRORS],
    err_n: usize,
    field: &'static [u8],
    code: &'static [u8],
    message: &'static [u8],
) -> usize {
    if err_n < MAX_ERRORS {
        errors[err_n] = ErrPart {
            field,
            code,
            message,
        };
        err_n + 1
    } else {
        err_n
    }
}

fn push_trace<'a>(traces: &mut [&'a [u8]; MAX_TRACE], trace_n: usize, t: &'a [u8]) -> usize {
    if trace_n < MAX_TRACE {
        traces[trace_n] = t;
        trace_n + 1
    } else {
        trace_n
    }
}

fn find_duplicate(existing: &[u8], title: &[u8], owner: &[u8], match_owner: bool) -> bool {
    let mut rest = existing;
    if rest.first() == Some(&b'[') {
        rest = &rest[1..];
    }
    loop {
        while rest
            .first()
            .is_some_and(|b| matches!(*b, b' ' | b'\n' | b'\t' | b','))
        {
            rest = &rest[1..];
        }
        if rest.is_empty() || rest.first() == Some(&b']') {
            return false;
        }
        if rest.first() != Some(&b'{') {
            return false;
        }
        let Some(end) = balanced_end(rest, b'{', b'}') else {
            return false;
        };
        let obj = &rest[..=end];
        let t = extract_string(obj, b"\"title\"");
        if t == title {
            if !match_owner || extract_string(obj, b"\"owner_id\"") == owner {
                return true;
            }
        }
        rest = &rest[end + 1..];
    }
}

fn write_invalid_config(out: &mut [u8], trace: &[u8]) -> usize {
    let mut i = 0usize;
    i = copy(out, i, b"{\"valid\":false,\"errors\":[],\"normalized\":null,\"reason_code\":\"invalid_config\",\"evaluation_trace\":");
    i = copy(out, i, trace);
    i = copy(out, i, b"}");
    i
}

fn write_ok(
    out: &mut [u8],
    title: &[u8],
    owner: &[u8],
    due: &[u8],
    traces: &[&[u8]],
) -> usize {
    let mut i = 0usize;
    i = copy(out, i, b"{\"valid\":true,\"errors\":[],\"normalized\":{\"title\":\"");
    i = copy(out, i, title);
    i = copy(out, i, b"\"");
    if !owner.is_empty() {
        i = copy(out, i, b",\"owner_id\":\"");
        i = copy(out, i, owner);
        i = copy(out, i, b"\"");
    }
    if !due.is_empty() {
        i = copy(out, i, b",\"due_date\":\"");
        i = copy(out, i, due);
        i = copy(out, i, b"\"");
    }
    i = copy(out, i, b"},\"reason_code\":\"ok\",\"evaluation_trace\":");
    i = write_string_array(out, i, traces);
    i = copy(out, i, b"}");
    i
}

fn write_fail(
    out: &mut [u8],
    reason: &[u8],
    errors: &[ErrPart],
    traces: &[&[u8]],
) -> usize {
    let mut i = 0usize;
    i = copy(out, i, b"{\"valid\":false,\"errors\":[");
    for (idx, err) in errors.iter().enumerate() {
        if idx > 0 {
            i = copy(out, i, b",");
        }
        i = copy(out, i, b"{\"field\":\"");
        i = copy(out, i, err.field);
        i = copy(out, i, b"\",\"code\":\"");
        i = copy(out, i, err.code);
        i = copy(out, i, b"\",\"message\":\"");
        i = copy(out, i, err.message);
        i = copy(out, i, b"\"}");
    }
    i = copy(out, i, b"],\"normalized\":null,\"reason_code\":\"");
    i = copy(out, i, reason);
    i = copy(out, i, b"\",\"evaluation_trace\":");
    i = write_string_array(out, i, traces);
    i = copy(out, i, b"}");
    i
}

fn write_string_array(out: &mut [u8], mut i: usize, items: &[&[u8]]) -> usize {
    i = copy(out, i, b"[");
    for (idx, item) in items.iter().enumerate() {
        if idx > 0 {
            i = copy(out, i, b",");
        }
        i = copy(out, i, b"\"");
        i = copy(out, i, item);
        i = copy(out, i, b"\"");
    }
    i = copy(out, i, b"]");
    i
}

fn object_after_key<'a>(hay: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let pos = find_key_at_depth(hay, key, 1)?;
    value_object_after(&hay[pos + key.len()..])
}

fn array_after_key<'a>(hay: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let pos = find_key_at_depth(hay, key, 1)?;
    value_array_after(&hay[pos + key.len()..])
}

fn value_object_after<'a>(after_key: &'a [u8]) -> Option<&'a [u8]> {
    let colon = after_key.iter().position(|b| *b == b':')?;
    let rest = skip_ws(&after_key[colon + 1..]);
    if rest.first() != Some(&b'{') {
        return None;
    }
    let end = balanced_end(rest, b'{', b'}')?;
    Some(&rest[..=end])
}

fn value_array_after<'a>(after_key: &'a [u8]) -> Option<&'a [u8]> {
    let colon = after_key.iter().position(|b| *b == b':')?;
    let rest = skip_ws(&after_key[colon + 1..]);
    if rest.first() != Some(&b'[') {
        return None;
    }
    let end = balanced_end(rest, b'[', b']')?;
    Some(&rest[..=end])
}

fn skip_ws(s: &[u8]) -> &[u8] {
    let mut rest = s;
    while rest
        .first()
        .is_some_and(|b| matches!(*b, b' ' | b'\n' | b'\t' | b'\r'))
    {
        rest = &rest[1..];
    }
    rest
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

fn extract_string<'a>(hay: &'a [u8], key: &[u8]) -> &'a [u8] {
    let Some(pos) = find(hay, key) else {
        return b"";
    };
    string_value_after(&hay[pos + key.len()..])
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
    let mut rest = skip_ws(&after_key[colon + 1..]);
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
    let rest = skip_ws(&after[colon + 1..]);
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

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
