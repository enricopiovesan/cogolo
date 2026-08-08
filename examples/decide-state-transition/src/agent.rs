//! platform.decide-state-transition — TOML-derived policy v1.2.
//!
//! Implements HP-01…HP-06 / UP-01…UP-08 from decide-state-transition-v1.1.0.toml
//! using explicit defaults in docs/lifecycle-and-approval-policy-v1.2.md.
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
static mut OUTPUT_BUF: [u8; 8192] = [0; 8192];

const POLICY: &[u8] = b"toml-derived-1.2.0";
const CONTRACT: &[u8] = b"1.2.0";
const AUTO_APPROVE_MAX: f64 = 100.0;

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
    let correlation = extract_string(input, b"\"correlation_id\"");
    let amount = extract_number(input, b"\"amount\"");
    let now = extract_string(input, b"\"now\"");
    let cancel_deadline = extract_string(input, b"\"cancel_deadline\"");
    let priority = extract_string(input, b"\"priority\"");

    let has_employee = has_role(input, b"employee");
    let has_manager = has_role(input, b"manager");
    let has_finance = has_role(input, b"finance");
    let has_support = has_role(input, b"support") || has_role(input, b"support_agent");
    let has_any_role = has_employee || has_manager || has_finance || has_support || has_role(input, b"customer")
        || has_role(input, b"compliance") || has_role(input, b"project_manager");

    let collected_legal = approval_collected(input, b"legal");
    let collected_finance = approval_collected(input, b"finance");
    let dual = flag_true(input, b"\"requires_dual_approval\"");
    let has_open_children = flag_true(input, b"\"has_open_children\"");

    let query = mode == b"query" || proposed.is_empty() || proposed == current;

    if !has_any_role {
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

    if query {
        let next = next_legal_states(entity_type, current, has_manager, has_finance);
        return write_query(out, next, correlation);
    }

    match entity_type {
        b"expense" => decide_expense(
            out,
            current,
            proposed,
            amount,
            has_manager,
            has_finance,
            dual,
            collected_legal,
            collected_finance,
            correlation,
        ),
        b"order" => decide_order(out, current, proposed, now, cancel_deadline, correlation),
        b"ticket" => decide_ticket(
            out,
            current,
            proposed,
            priority,
            has_open_children,
            correlation,
        ),
        _ => write_decision(
            out,
            false,
            b"denied",
            b"UNKNOWN_ENTITY_TYPE",
            b"Unrecognised entity_type for this policy.",
            b"[]",
            b"error",
            b"Unknown entity type",
            correlation,
        ),
    }
}

fn next_legal_states(
    entity_type: &[u8],
    current: &[u8],
    has_manager: bool,
    has_finance: bool,
) -> &'static [u8] {
    match (entity_type, current) {
        (b"expense", b"draft") => br#"["submitted"]"#,
        (b"expense", b"submitted") if has_manager || has_finance => br#"["approved"]"#,
        (b"expense", b"submitted") => br#"[]"#,
        (b"order", b"placed") => br#"["cancelled"]"#,
        (b"ticket", b"open") => br#"["escalated","closed"]"#,
        _ => br#"[]"#,
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn decide_expense(
    out: &mut [u8],
    current: &[u8],
    proposed: &[u8],
    amount: Option<f64>,
    has_manager: bool,
    has_finance: bool,
    dual: bool,
    collected_legal: bool,
    collected_finance: bool,
    correlation: &[u8],
) -> usize {
    if current == b"draft" && proposed == b"submitted" {
        let Some(value) = amount else {
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
        };
        if value >= AUTO_APPROVE_MAX {
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

    if current == b"submitted" && proposed == b"approved" {
        if dual {
            let missing_legal = !collected_legal;
            let missing_finance = !collected_finance;
            if missing_legal || missing_finance {
                let approvals: &[u8] = match (missing_legal, missing_finance) {
                    (true, true) => {
                        br#"[{"role":"legal","min_count":1,"logic":"all","reason":"dual approval"},{"role":"finance","min_count":1,"logic":"all","reason":"dual approval"}]"#
                    }
                    (true, false) => {
                        br#"[{"role":"legal","min_count":1,"logic":"all","reason":"dual approval"}]"#
                    }
                    (false, true) => {
                        br#"[{"role":"finance","min_count":1,"logic":"all","reason":"dual approval"}]"#
                    }
                    _ => b"[]",
                };
                return write_decision(
                    out,
                    false,
                    b"requires_approval",
                    b"PARTIAL_APPROVALS",
                    b"One or more required approvals are still missing.",
                    approvals,
                    b"warning",
                    b"Approvals incomplete",
                    correlation,
                );
            }
            return write_decision(
                out,
                true,
                b"allowed",
                b"DUAL_APPROVAL_COMPLETE",
                b"Legal and finance approvals are both present.",
                b"[]",
                b"info",
                b"",
                correlation,
            );
        }

        if has_finance {
            return write_decision(
                out,
                true,
                b"allowed",
                b"APPROVED_BY_FINANCE",
                b"Finance authority completed the approval.",
                b"[]",
                b"info",
                b"",
                correlation,
            );
        }
        if has_manager {
            return write_decision(
                out,
                false,
                b"requires_approval",
                b"AMOUNT_EXCEEDS_LIMIT",
                b"Manager acknowledged; finance approval is still required.",
                br#"[{"role":"finance","min_count":1,"logic":"all","reason":"high-value expense"}]"#,
                b"warning",
                b"Requires Finance",
                correlation,
            );
        }
        return write_decision(
            out,
            false,
            b"denied",
            b"INSUFFICIENT_ROLE",
            b"Only managers or finance can approve a submitted expense.",
            b"[]",
            b"error",
            b"Insufficient role",
            correlation,
        );
    }

    if current.is_empty() || proposed.is_empty() {
        return write_decision(
            out,
            false,
            b"denied",
            b"UNKNOWN_STATE",
            b"Unrecognised state combination for expense.",
            b"[]",
            b"error",
            b"Unknown state",
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

unsafe fn decide_order(
    out: &mut [u8],
    current: &[u8],
    proposed: &[u8],
    now: &[u8],
    cancel_deadline: &[u8],
    correlation: &[u8],
) -> usize {
    if current == b"fulfilled" && proposed == b"draft" {
        return write_decision(
            out,
            false,
            b"denied",
            b"ILLEGAL_TRANSITION",
            b"The requested state jump is not legal for this entity lifecycle.",
            b"[]",
            b"error",
            b"Illegal transition",
            correlation,
        );
    }

    if current == b"placed" && proposed == b"cancelled" {
        if now.is_empty() || cancel_deadline.is_empty() {
            return write_decision(
                out,
                false,
                b"requires_additional_info",
                b"MISSING_CANCEL_WINDOW",
                b"now and cancel_deadline are required to evaluate cancellation.",
                b"[]",
                b"warning",
                b"Window data required",
                correlation,
            );
        }
        if bytes_le(now, cancel_deadline) {
            return write_decision(
                out,
                true,
                b"allowed",
                b"CANCEL_WITHIN_WINDOW",
                b"Cancellation is still inside the allowed window.",
                b"[]",
                b"info",
                b"",
                correlation,
            );
        }
        return write_decision(
            out,
            false,
            b"denied",
            b"CANCEL_WINDOW_EXPIRED",
            b"The cancellation window has expired.",
            b"[]",
            b"error",
            b"Window expired",
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

unsafe fn decide_ticket(
    out: &mut [u8],
    current: &[u8],
    proposed: &[u8],
    priority: &[u8],
    has_open_children: bool,
    correlation: &[u8],
) -> usize {
    if current == b"open" && proposed == b"escalated" {
        if priority == b"critical" {
            return write_decision(
                out,
                true,
                b"allowed",
                b"PRIORITY_ESCALATION",
                b"Critical priority tickets may escalate from open.",
                b"[]",
                b"info",
                b"",
                correlation,
            );
        }
        return write_decision(
            out,
            false,
            b"denied",
            b"PRIORITY_NOT_CRITICAL",
            b"Escalation requires context.priority to be critical.",
            b"[]",
            b"error",
            b"Not critical",
            correlation,
        );
    }

    if current == b"open" && proposed == b"closed" {
        if has_open_children {
            return write_decision(
                out,
                false,
                b"denied",
                b"HAS_OPEN_CHILDREN",
                b"Parent ticket cannot close while child tickets remain open.",
                b"[]",
                b"error",
                b"Open children",
                correlation,
            );
        }
        return write_decision(
            out,
            true,
            b"allowed",
            b"TICKET_CLOSED",
            b"Ticket may close with no open children.",
            b"[]",
            b"info",
            b"",
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

/// Lexicographic compare suitable for identical-format ISO-8601 Zulu timestamps.
fn bytes_le(a: &[u8], b: &[u8]) -> bool {
    a <= b
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
        br#","suggested_actions":[],"ui_hints":{"severity":"info"},"contract_version":""#,
    );
    i = copy(out, i, CONTRACT);
    i = copy(out, i, br#"","policy_version":""#);
    i = copy(out, i, POLICY);
    i = copy(out, i, br#"""#);
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
    i = copy(
        out,
        i,
        br#","next_legal_states":[],"suggested_actions":[],"ui_hints":{"severity":""#,
    );
    i = copy(out, i, severity);
    if !badge.is_empty() {
        i = copy(out, i, br#"","badge":""#);
        i = copy(out, i, badge);
    }
    i = copy(out, i, br#""},"contract_version":""#);
    i = copy(out, i, CONTRACT);
    i = copy(out, i, br#"","policy_version":""#);
    i = copy(out, i, POLICY);
    i = copy(out, i, br#"""#);
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

fn has_role(hay: &[u8], role: &[u8]) -> bool {
    let Some(roles) = json_array_after_key(hay, b"\"roles\"") else {
        return false;
    };
    let mut needle = [0u8; 64];
    if role.len() + 2 > needle.len() {
        return false;
    }
    needle[0] = b'"';
    needle[1..1 + role.len()].copy_from_slice(role);
    needle[1 + role.len()] = b'"';
    contains(&roles, &needle[..role.len() + 2])
}

fn approval_collected(hay: &[u8], role: &[u8]) -> bool {
    let Some(arr) = json_array_after_key(hay, b"\"already_collected_approvals\"") else {
        return false;
    };
    // Match "role":"<role>" inside the approvals array.
    let mut needle = [0u8; 80];
    let prefix = br#""role":""#;
    if prefix.len() + role.len() + 1 > needle.len() {
        return false;
    }
    needle[..prefix.len()].copy_from_slice(prefix);
    needle[prefix.len()..prefix.len() + role.len()].copy_from_slice(role);
    needle[prefix.len() + role.len()] = b'"';
    contains(&arr, &needle[..prefix.len() + role.len() + 1])
}

fn flag_true(hay: &[u8], key: &[u8]) -> bool {
    let Some(pos) = find(hay, key) else {
        return false;
    };
    let after = &hay[pos + key.len()..];
    let Some(colon) = after.iter().position(|b| *b == b':') else {
        return false;
    };
    let mut rest = &after[colon + 1..];
    while rest.first() == Some(&b' ') || rest.first() == Some(&b'\n') || rest.first() == Some(&b'\t')
    {
        rest = &rest[1..];
    }
    rest.starts_with(b"true")
}

fn json_array_after_key<'a>(hay: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let pos = find(hay, key)?;
    let after = &hay[pos + key.len()..];
    let colon = after.iter().position(|b| *b == b':')?;
    let mut rest = &after[colon + 1..];
    while rest.first() == Some(&b' ') || rest.first() == Some(&b'\n') || rest.first() == Some(&b'\t')
    {
        rest = &rest[1..];
    }
    if rest.first() != Some(&b'[') {
        return None;
    }
    let mut depth = 0i32;
    for (i, b) in rest.iter().enumerate() {
        match *b {
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&rest[..=i]);
                }
            }
            _ => {}
        }
    }
    None
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
    while end < rest.len() && ((rest[end] >= b'0' && rest[end] <= b'9') || rest[end] == b'.') {
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
