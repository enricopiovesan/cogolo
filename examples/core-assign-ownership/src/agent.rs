//! core.assign-ownership — pure owner resolution against workspace members.
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
static mut OUTPUT_BUF: [u8; 4096] = [0; 4096];

const MAX_TRACE: usize = 4;

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
    let members = array_after_key(input, b"\"workspace_members\"").unwrap_or(b"[]");
    let config = object_after_key(input, b"\"ownership_config\"").unwrap_or(b"");
    let creator = extract_string_at_depth(input, b"\"creator_id\"", 1);
    let suggested_null = is_null_at_depth(input, b"\"suggested_owner\"", 1);
    let suggested = if suggested_null {
        b""
    } else {
        extract_string_at_depth(input, b"\"suggested_owner\"", 1)
    };

    if config.is_empty() {
        return write_result(
            out,
            None,
            b"config_error",
            b"config_error",
            &[b"ownership_config missing"],
        );
    }

    let fallback = extract_string(config, b"\"fallback\"");
    let fallback = if fallback.is_empty() {
        b"creator"
    } else {
        fallback
    };
    if !matches!(fallback, b"creator" | b"unassigned" | b"fail") {
        return write_result(
            out,
            None,
            b"config_error",
            b"config_error",
            &[b"invalid fallback"],
        );
    }
    let require_active = extract_bool(config, b"\"require_active_member\"").unwrap_or(true);

    let mut traces: [&[u8]; MAX_TRACE] = [b""; MAX_TRACE];
    let mut trace_n = 0usize;

    if suggested_null || suggested.is_empty() {
        traces[trace_n] = b"suggested_owner null";
        trace_n += 1;
        return apply_fallback(out, fallback, creator, members, require_active, &mut traces, trace_n);
    }

    if let Some((id, method, note)) = resolve_member(members, suggested, require_active) {
        if method == b"inactive" {
            traces[0] = note;
            return write_result(out, None, b"unresolved", b"inactive_member", &traces[..1]);
        }
        traces[0] = note;
        return write_result(out, Some(id), method, b"ok", &traces[..1]);
    }

    traces[0] = b"no member match for suggestion";
    trace_n = 1;
    apply_fallback(out, fallback, creator, members, require_active, &mut traces, trace_n)
}

fn apply_fallback<'a>(
    out: &mut [u8],
    fallback: &[u8],
    creator: &'a [u8],
    members: &[u8],
    require_active: bool,
    traces: &mut [&'a [u8]; MAX_TRACE],
    mut trace_n: usize,
) -> usize {
    match fallback {
        b"creator" => {
            if creator.is_empty() {
                if trace_n < MAX_TRACE {
                    traces[trace_n] = b"fallback=creator but creator_id missing";
                    trace_n += 1;
                }
                return write_result(out, None, b"unresolved", b"unresolved", &traces[..trace_n]);
            }
            if require_active {
                if let Some((id, method, _)) = resolve_member(members, creator, true) {
                    if method != b"inactive" {
                        if trace_n < MAX_TRACE {
                            traces[trace_n] = b"fallback=creator";
                            trace_n += 1;
                        }
                        return write_result(
                            out,
                            Some(id),
                            b"fallback_creator",
                            b"ok",
                            &traces[..trace_n],
                        );
                    }
                }
                // Creator id may not be listed; still accept when present as string id.
            }
            if trace_n < MAX_TRACE {
                traces[trace_n] = b"fallback=creator";
                trace_n += 1;
            }
            write_result(
                out,
                Some(creator),
                b"fallback_creator",
                b"ok",
                &traces[..trace_n],
            )
        }
        b"unassigned" => {
            if trace_n < MAX_TRACE {
                traces[trace_n] = b"fallback=unassigned";
                trace_n += 1;
            }
            write_result(
                out,
                None,
                b"fallback_unassigned",
                b"ok",
                &traces[..trace_n],
            )
        }
        _ => {
            if trace_n < MAX_TRACE {
                traces[trace_n] = b"fallback=fail";
                trace_n += 1;
            }
            write_result(out, None, b"unresolved", b"unresolved", &traces[..trace_n])
        }
    }
}

/// Returns (id, method, note). method == b"inactive" means matched but inactive.
fn resolve_member<'a>(
    members: &'a [u8],
    suggestion: &[u8],
    require_active: bool,
) -> Option<(&'a [u8], &'static [u8], &'static [u8])> {
    let mut rest = members;
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
            return None;
        }
        if rest.first() != Some(&b'{') {
            return None;
        }
        let end = balanced_end(rest, b'{', b'}')?;
        let obj = &rest[..=end];
        let id = extract_string(obj, b"\"id\"");
        let name = extract_string(obj, b"\"name\"");
        let email = extract_string(obj, b"\"email\"");
        let active = extract_bool(obj, b"\"active\"").unwrap_or(true);

        let mut method: Option<&'static [u8]> = None;
        let mut note: Option<&'static [u8]> = None;
        if !id.is_empty() && eq_ascii_ci(id, suggestion) {
            method = Some(b"id_match");
            note = Some(b"id match");
        } else if !email.is_empty() && eq_ascii_ci(email, suggestion) {
            method = Some(b"email_match");
            note = Some(b"email match");
        } else if !name.is_empty() && eq_ascii_ci(name, suggestion) {
            method = Some(b"name_match");
            note = Some(b"exact name match");
        }

        if let (Some(m), Some(n)) = (method, note) {
            if require_active && !active {
                return Some((id, b"inactive", b"matched inactive member"));
            }
            return Some((id, m, n));
        }
        rest = &rest[end + 1..];
    }
}

fn write_result(
    out: &mut [u8],
    owner_id: Option<&[u8]>,
    method: &[u8],
    reason: &[u8],
    traces: &[&[u8]],
) -> usize {
    let mut i = 0usize;
    i = copy(out, i, b"{\"owner_id\":");
    if let Some(id) = owner_id {
        i = copy(out, i, b"\"");
        i = copy(out, i, id);
        i = copy(out, i, b"\"");
    } else {
        i = copy(out, i, b"null");
    }
    i = copy(out, i, b",\"resolution_method\":\"");
    i = copy(out, i, method);
    i = copy(out, i, b"\",\"reason_code\":\"");
    i = copy(out, i, reason);
    i = copy(out, i, b"\",\"evaluation_trace\":[");
    for (idx, t) in traces.iter().enumerate() {
        if idx > 0 {
            i = copy(out, i, b",");
        }
        i = copy(out, i, b"\"");
        i = copy(out, i, t);
        i = copy(out, i, b"\"");
    }
    i = copy(out, i, b"]}");
    i
}

fn eq_ascii_ci(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .all(|(x, y)| x.to_ascii_lowercase() == y.to_ascii_lowercase())
}

fn is_null_at_depth(hay: &[u8], key: &[u8], depth: i32) -> bool {
    let Some(pos) = find_key_at_depth(hay, key, depth) else {
        return false;
    };
    let after = &hay[pos + key.len()..];
    let Some(colon) = after.iter().position(|b| *b == b':') else {
        return false;
    };
    let rest = skip_ws(&after[colon + 1..]);
    rest.starts_with(b"null")
}

fn object_after_key<'a>(hay: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let pos = find_key_at_depth(hay, key, 1)?;
    let after = &hay[pos + key.len()..];
    let colon = after.iter().position(|b| *b == b':')?;
    let rest = skip_ws(&after[colon + 1..]);
    if rest.first() != Some(&b'{') {
        return None;
    }
    let end = balanced_end(rest, b'{', b'}')?;
    Some(&rest[..=end])
}

fn array_after_key<'a>(hay: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let pos = find_key_at_depth(hay, key, 1)?;
    let after = &hay[pos + key.len()..];
    let colon = after.iter().position(|b| *b == b':')?;
    let rest = skip_ws(&after[colon + 1..]);
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
