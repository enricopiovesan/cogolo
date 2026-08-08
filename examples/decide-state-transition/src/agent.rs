//! Minimal ABI-clean probe for platform.decide-state-transition.
//!
//! Implements a tiny embedded expense policy sufficient for HP-01 / UP-01 / UP-06
//! so the authoring path can be exercised end-to-end. Full policy tables from
//! `docs/lifecycle-and-approval-policy-v1.1.md` are not present in the starting
//! TOML and are intentionally not invented here.
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
        let input = &INPUT_BUF[..total];
        let out_len = decide(input, &mut OUTPUT_BUF);
        let out = IoVec {
            buffer: OUTPUT_BUF.as_ptr(),
            length: out_len,
        };
        let mut written = 0usize;
        let _ = fd_write(1, &out, 1, &mut written);
    }
}

unsafe fn decide(input: &[u8], out: &mut [u8]) -> usize {
    let entity_type = extract_string(input, b"\"entity_type\"");
    let current = extract_string(input, b"\"current_state\"");
    let proposed = extract_string(input, b"\"proposed_state\"");
    let mode = extract_string(input, b"\"mode\"");
    let has_role = contains(input, b"\"roles\":[\"") || contains(input, b"\"roles\": [\"");
    let roles_empty = !has_role;
    let amount = extract_number(input, b"\"amount\"");
    let correlation = extract_string(input, b"\"correlation_id\"");

    let query = mode == b"query" || proposed.is_empty() || proposed == current;

    if roles_empty {
        return write_decision(
            out,
            false,
            b"denied",
            b"ACTOR_HAS_NO_ROLES",
            b"No roles are present on the actor.",
            b"[]",
            b"error",
            b"Actor has no roles",
            correlation,
        );
    }

    if entity_type != b"expense" {
        return write_decision(
            out,
            false,
            b"denied",
            b"UNKNOWN_ENTITY_TYPE",
            b"Unrecognised entity_type for this probe policy.",
            b"[]",
            b"error",
            b"Unknown entity type",
            correlation,
        );
    }

    if query {
        let next: &[u8] = match current {
            b"draft" => br#"["submitted"]"#,
            b"submitted" => br#"["approved","rejected"]"#,
            _ => br#"[]"#,
        };
        return write_query(out, next, correlation);
    }

    if current == b"draft" && proposed == b"submitted" {
        if let Some(value) = amount {
            if value >= 100.0 {
                return write_decision(
                    out,
                    false,
                    b"requires_approval",
                    b"AMOUNT_EXCEEDS_LIMIT",
                    b"Amount exceeds auto-approval threshold; finance approval required.",
                    br#"[{"role":"finance","min_count":1,"logic":"all","reason":"high-value expense"}]"#,
                    b"warning",
                    b"Requires Finance",
                    correlation,
                );
            }
            return write_decision(
                out,
                true,
                b"allowed",
                b"AUTO_APPROVED",
                b"Low-value expense may transition from draft to submitted.",
                b"[]",
                b"info",
                b"",
                correlation,
            );
        }
        return write_decision(
            out,
            false,
            b"requires_additional_info",
            b"MISSING_AMOUNT",
            b"Amount is required in context for this transition.",
            b"[]",
            b"warning",
            b"Amount required",
            correlation,
        );
    }

    write_decision(
        out,
        false,
        b"denied",
        b"ILLEGAL_TRANSITION",
        b"The requested state jump is not legal for this entity lifecycle.",
        b"[]",
        b"error",
        b"Illegal transition",
        correlation,
    )
}

unsafe fn write_query(out: &mut [u8], next: &[u8], correlation: &[u8]) -> usize {
    let mut i = 0usize;
    i = copy(
        out,
        i,
        br#"{"allowed":false,"decision":"denied","reasons":[{"code":"QUERY_ONLY","message":"Query mode returns next legal states only.","severity":"info"}],"required_approvals":[],"next_legal_states":"#,
    );
    i = copy(out, i, next);
    i = copy(
        out,
        i,
        br#","suggested_actions":[],"ui_hints":{"severity":"info"},"contract_version":"1.1.0","policy_version":"probe-1.0.0""#,
    );
    if !correlation.is_empty() {
        i = copy(out, i, br#","correlation_id":""#);
        i = copy(out, i, correlation);
        i = copy(out, i, br#"""#);
    }
    i = copy(out, i, br#"}"#);
    i
}

#[allow(clippy::too_many_arguments)]
unsafe fn write_decision(
    out: &mut [u8],
    allowed: bool,
    decision: &[u8],
    code: &[u8],
    message: &[u8],
    required_approvals: &[u8],
    severity: &[u8],
    badge: &[u8],
    correlation: &[u8],
) -> usize {
    let mut i = 0usize;
    i = copy(out, i, br#"{"allowed":"#);
    i = copy(out, i, if allowed { b"true" } else { b"false" });
    i = copy(out, i, br#","decision":""#);
    i = copy(out, i, decision);
    i = copy(out, i, br#"","reasons":[{"code":""#);
    i = copy(out, i, code);
    i = copy(out, i, br#"","message":""#);
    i = copy(out, i, message);
    i = copy(out, i, br#"","severity":""#);
    i = copy(out, i, severity);
    i = copy(out, i, br#""}],"required_approvals":"#);
    i = copy(out, i, required_approvals);
    i = copy(out, i, br#","next_legal_states":[],"suggested_actions":[],"ui_hints":{"severity":""#);
    i = copy(out, i, severity);
    if !badge.is_empty() {
        i = copy(out, i, br#"","badge":""#);
        i = copy(out, i, badge);
    }
    i = copy(
        out,
        i,
        br#""},"contract_version":"1.1.0","policy_version":"probe-1.0.0""#,
    );
    if !correlation.is_empty() {
        i = copy(out, i, br#","correlation_id":""#);
        i = copy(out, i, correlation);
        i = copy(out, i, br#"""#);
    }
    i = copy(out, i, br#"}"#);
    i
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
    hay.windows(needle.len()).any(|window| window == needle)
}

fn extract_string<'a>(hay: &'a [u8], key: &[u8]) -> &'a [u8] {
    let Some(pos) = find(hay, key) else {
        return b"";
    };
    let after = &hay[pos + key.len()..];
    let Some(colon) = after.iter().position(|b| *b == b':') else {
        return b"";
    };
    let mut rest = &after[colon + 1..];
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

fn extract_number(hay: &[u8], key: &[u8]) -> Option<f64> {
    let pos = find(hay, key)?;
    let after = &hay[pos + key.len()..];
    let colon = after.iter().position(|b| *b == b':')?;
    let mut rest = &after[colon + 1..];
    while rest.first() == Some(&b' ') {
        rest = &rest[1..];
    }
    let mut end = 0usize;
    while end < rest.len()
        && ((rest[end] >= b'0' && rest[end] <= b'9') || rest[end] == b'.')
    {
        end += 1;
    }
    if end == 0 {
        return None;
    }
    parse_f64(&rest[..end])
}

fn parse_f64(bytes: &[u8]) -> Option<f64> {
    let mut value = 0.0f64;
    let mut frac = 0.0f64;
    let mut place = 0.1f64;
    let mut seen_dot = false;
    for &b in bytes {
        if b == b'.' {
            if seen_dot {
                return None;
            }
            seen_dot = true;
            continue;
        }
        if b < b'0' || b > b'9' {
            return None;
        }
        let digit = f64::from(b - b'0');
        if seen_dot {
            frac += digit * place;
            place *= 0.1;
        } else {
            value = value * 10.0 + digit;
        }
    }
    Some(value + frac)
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len())
        .position(|window| window == needle)
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
