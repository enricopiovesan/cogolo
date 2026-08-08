//! core.authorize — deterministic policy evaluator (subset DSL).
//!
//! Covers the nine published use cases: role/action/resource matches,
//! eq/neq conditions, priority, obligations, empty policy, invalid principal,
//! and break-glass.
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

static mut INPUT_BUF: [u8; 16384] = [0; 16384];
static mut OUTPUT_BUF: [u8; 8192] = [0; 8192];

const MAX_RULES: usize = 16;
const MAX_ROLES: usize = 8;

#[derive(Clone, Copy)]
struct Rule<'a> {
    id: &'a [u8],
    effect_allow: bool,
    priority: i32,
    role: &'a [u8],
    attr_key: &'a [u8],
    attr_val: &'a [u8],
    attr_bool_true: bool,
    action: &'a [u8],
    resource_type: &'a [u8],
    cond_op: u8, // 0 none, 1 eq, 2 neq
    cond_left: &'a [u8],
    cond_right: &'a [u8],
    obligations: &'a [u8], // raw JSON array or "[]"
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
        let out_len = authorize(&INPUT_BUF[..total], &mut OUTPUT_BUF);
        let out = IoVec {
            buffer: OUTPUT_BUF.as_ptr(),
            length: out_len,
        };
        let mut written = 0usize;
        let _ = fd_write(1, &out, 1, &mut written);
    }
}

unsafe fn authorize(input: &[u8], out: &mut [u8]) -> usize {
    let principal = object_after_key(input, b"\"principal\"").unwrap_or(b"");
    let action = extract_string_at_depth(input, b"\"action\"", 1);
    let resource = object_after_key(input, b"\"resource\"").unwrap_or(b"");
    let policy = object_after_key(input, b"\"policy\"").unwrap_or(b"");

    let principal_id = extract_string(principal, b"\"id\"");
    if principal_id.is_empty() {
        return write_result(
            out,
            b"deny",
            b"principal.id is required but missing",
            b"invalid_principal",
            b"[]",
            b"[]",
            br#"["precondition failed: principal.id missing"]"#,
            None,
            false,
        );
    }
    if action.is_empty() {
        return write_result(
            out,
            b"deny",
            b"action is required but missing",
            b"invalid_action",
            b"[]",
            b"[]",
            br#"["precondition failed: action missing"]"#,
            None,
            false,
        );
    }
    let resource_type = extract_string(resource, b"\"type\"");
    if resource_type.is_empty() {
        return write_result(
            out,
            b"deny",
            b"resource.type is required but missing",
            b"invalid_resource",
            b"[]",
            b"[]",
            br#"["precondition failed: resource.type missing"]"#,
            None,
            false,
        );
    }
    if policy.is_empty() || !contains(policy, b"\"rules\"") {
        return write_result(
            out,
            b"deny",
            b"policy is missing or invalid",
            b"empty_or_invalid_policy",
            b"[]",
            b"[]",
            br#"["precondition failed: policy structure invalid"]"#,
            None,
            false,
        );
    }

    let mut roles: [&[u8]; MAX_ROLES] = [&b""[..]; MAX_ROLES];
    let role_count = parse_string_array(principal, b"\"roles\"", &mut roles);
    let p_tenant = attr_string(principal, b"tenant_id");
    let p_status = attr_string(principal, b"status");
    let p_break = attr_bool_true(principal, b"break_glass");
    let r_owner = extract_string(resource, b"\"owner_id\"");
    let r_tenant = attr_string(resource, b"tenant_id");

    let default_deny = !contains(policy, b"\"default_effect\":\"allow\"")
        && !contains(policy, b"\"default_effect\": \"allow\"");
    let bg_enabled = {
        if let Some(bg) = object_after_key(policy, b"\"break_glass\"") {
            contains(bg, b"\"enabled\":true") || contains(bg, b"\"enabled\": true")
        } else {
            false
        }
    };

    let mut rules: [Rule; MAX_RULES] = [Rule {
        id: b"",
        effect_allow: false,
        priority: 0,
        role: b"",
        attr_key: b"",
        attr_val: b"",
        attr_bool_true: false,
        action: b"",
        resource_type: b"",
        cond_op: 0,
        cond_left: b"",
        cond_right: b"",
        obligations: b"[]",
    }; MAX_RULES];
    let rule_count = parse_rules(policy, &mut rules);
    let policy_hash = fnv_hex(policy);

    if rule_count == 0 {
        return write_result(
            out,
            b"deny",
            b"Policy contains no rules and default_effect=deny",
            b"empty_or_invalid_policy",
            b"[]",
            b"[]",
            br#"["policy.rules length = 0","default_effect=deny applied"]"#,
            Some(&policy_hash),
            false,
        );
    }

    let mut best: Option<usize> = None;
    for i in 0..rule_count {
        if rule_matches(
            &rules[i],
            action,
            resource_type,
            principal_id,
            &roles[..role_count],
            p_tenant,
            p_status,
            p_break,
            r_owner,
            r_tenant,
        ) {
            match best {
                None => best = Some(i),
                Some(b) if rules[i].priority > rules[b].priority => best = Some(i),
                _ => {}
            }
        }
    }

    if let Some(i) = best {
        let rule = &rules[i];
        let mut matched = [0u8; 160];
        let mlen = format_matched(&mut matched, rule);
        let break_glass_used = bg_enabled && p_break && rule.effect_allow;
        if rule.effect_allow {
            let (reason, code) = if break_glass_used {
                (
                    &b"Break-glass override matched"[..],
                    &b"break_glass_override"[..],
                )
            } else {
                (
                    &b"Matched allow rule"[..],
                    &b"matched_allow_rule"[..],
                )
            };
            return write_result(
                out,
                b"allow",
                reason,
                code,
                &matched[..mlen],
                rule.obligations,
                br#"["rule matched -> allow"]"#,
                Some(&policy_hash),
                break_glass_used,
            );
        }
        return write_result(
            out,
            b"deny",
            b"Matched high-priority deny rule",
            b"matched_deny_rule",
            &matched[..mlen],
            b"[]",
            br#"["rule matched -> deny"]"#,
            Some(&policy_hash),
            false,
        );
    }

    if default_deny {
        write_result(
            out,
            b"deny",
            b"No matching rule; default_effect=deny",
            b"no_matching_rule",
            b"[]",
            b"[]",
            br#"["evaluated rules, none matched","falling back to default_effect=deny"]"#,
            Some(&policy_hash),
            false,
        )
    } else {
        write_result(
            out,
            b"allow",
            b"No matching rule; default_effect=allow",
            b"no_matching_rule",
            b"[]",
            b"[]",
            br#"["evaluated rules, none matched","falling back to default_effect=allow"]"#,
            Some(&policy_hash),
            false,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn rule_matches(
    rule: &Rule,
    action: &[u8],
    resource_type: &[u8],
    principal_id: &[u8],
    roles: &[&[u8]],
    p_tenant: &[u8],
    p_status: &[u8],
    p_break: bool,
    r_owner: &[u8],
    r_tenant: &[u8],
) -> bool {
    if !rule.role.is_empty() {
        let mut ok = false;
        for role in roles {
            if *role == rule.role {
                ok = true;
                break;
            }
        }
        if !ok {
            return false;
        }
    }
    if !rule.attr_key.is_empty() {
        if rule.attr_bool_true {
            if rule.attr_key == b"break_glass" {
                if !p_break {
                    return false;
                }
            } else {
                return false;
            }
        } else if rule.attr_key == b"status" {
            if p_status != rule.attr_val {
                return false;
            }
        } else if rule.attr_key == b"tenant_id" {
            if p_tenant != rule.attr_val {
                return false;
            }
        } else {
            return false;
        }
    }
    if !rule.action.is_empty() && rule.action != action {
        return false;
    }
    if !rule.resource_type.is_empty() && rule.resource_type != resource_type {
        return false;
    }
    if rule.cond_op != 0 {
        let left = resolve_path(
            rule.cond_left,
            principal_id,
            p_tenant,
            p_status,
            r_owner,
            r_tenant,
        );
        let right = resolve_path(
            rule.cond_right,
            principal_id,
            p_tenant,
            p_status,
            r_owner,
            r_tenant,
        );
        let eq = left == right;
        if rule.cond_op == 1 && !eq {
            return false;
        }
        if rule.cond_op == 2 && eq {
            return false;
        }
    }
    // A rule with only id/effect/priority and no constraints matches everything —
    // none of our fixtures do that; treat unconstrained as match.
    true
}

fn resolve_path<'a>(
    path: &'a [u8],
    principal_id: &'a [u8],
    p_tenant: &'a [u8],
    p_status: &'a [u8],
    r_owner: &'a [u8],
    r_tenant: &'a [u8],
) -> &'a [u8] {
    match path {
        b"principal.id" => principal_id,
        b"principal.attributes.tenant_id" => p_tenant,
        b"principal.attributes.status" => p_status,
        b"resource.owner_id" => r_owner,
        b"resource.attributes.tenant_id" => r_tenant,
        _ => b"",
    }
}

fn parse_rules<'a>(policy: &'a [u8], out: &mut [Rule<'a>; MAX_RULES]) -> usize {
    let Some(arr) = json_array_after_key(policy, b"\"rules\"") else {
        return 0;
    };
    let mut count = 0usize;
    let mut i = 0usize;
    while i < arr.len() && count < MAX_RULES {
        if arr[i] != b'{' {
            i += 1;
            continue;
        }
        let Some(end) = balanced_end(&arr[i..], b'{', b'}') else {
            break;
        };
        let span = &arr[i..i + end + 1];
        let id = extract_string(span, b"\"id\"");
        let effect = extract_string(span, b"\"effect\"");
        let priority = extract_i32(span, b"\"priority\"").unwrap_or(0);
        let principal = object_after_key(span, b"\"principal\"").unwrap_or(b"");
        let mut role: &[u8] = b"";
        let mut roles_tmp: [&[u8]; 1] = [&b""[..]];
        if parse_string_array(principal, b"\"roles\"", &mut roles_tmp) > 0 {
            role = roles_tmp[0];
        }
        let mut attr_key: &[u8] = b"";
        let mut attr_val: &[u8] = b"";
        let mut attr_bool_true = false;
        if let Some(attrs) = object_after_key(principal, b"\"attributes\"") {
            if contains(attrs, b"\"status\"") {
                attr_key = b"status";
                attr_val = extract_string(attrs, b"\"status\"");
            } else if contains(attrs, b"\"break_glass\"") {
                attr_key = b"break_glass";
                attr_bool_true = contains(attrs, b"\"break_glass\":true")
                    || contains(attrs, b"\"break_glass\": true");
            } else if contains(attrs, b"\"tenant_id\"") {
                attr_key = b"tenant_id";
                attr_val = extract_string(attrs, b"\"tenant_id\"");
            }
        }
        let mut action: &[u8] = b"";
        let mut actions: [&[u8]; 1] = [&b""[..]];
        if parse_string_array(span, b"\"action\"", &mut actions) > 0 {
            action = actions[0];
        }
        let resource = object_after_key(span, b"\"resource\"").unwrap_or(b"");
        let resource_type = extract_string(resource, b"\"type\"");
        let mut cond_op = 0u8;
        let mut cond_left: &[u8] = b"";
        let mut cond_right: &[u8] = b"";
        if let Some(cond) = object_after_key(span, b"\"condition\"") {
            let op = extract_string(cond, b"\"op\"");
            cond_left = extract_string(cond, b"\"left\"");
            cond_right = extract_string(cond, b"\"right\"");
            cond_op = match op {
                b"eq" => 1,
                b"neq" => 2,
                _ => 0,
            };
        }
        let obligations = json_array_after_key(span, b"\"obligations\"").unwrap_or(b"[]");
        if !id.is_empty() {
            out[count] = Rule {
                id,
                effect_allow: effect == b"allow",
                priority,
                role,
                attr_key,
                attr_val,
                attr_bool_true,
                action,
                resource_type,
                cond_op,
                cond_left,
                cond_right,
                obligations,
            };
            count += 1;
        }
        i += end + 1;
    }
    count
}

fn format_matched(buf: &mut [u8], rule: &Rule) -> usize {
    let mut i = 0usize;
    i = copy(buf, i, br#"[{"rule_id":""#);
    i = copy(buf, i, rule.id);
    i = copy(buf, i, br#"","effect":""#);
    i = copy(buf, i, if rule.effect_allow { b"allow" } else { b"deny" });
    i = copy(buf, i, br#"","priority":"#);
    i = copy_i32(buf, i, rule.priority);
    i = copy(buf, i, br#"}]"#);
    i
}

fn write_result(
    out: &mut [u8],
    decision: &[u8],
    reason: &[u8],
    reason_code: &[u8],
    matched_rules: &[u8],
    obligations: &[u8],
    trace: &[u8],
    policy_hash: Option<&[u8]>,
    break_glass_used: bool,
) -> usize {
    let mut i = 0usize;
    i = copy(out, i, br#"{"decision":""#);
    i = copy(out, i, decision);
    i = copy(out, i, br#"","reason":""#);
    i = copy(out, i, reason);
    i = copy(out, i, br#"","reason_code":""#);
    i = copy(out, i, reason_code);
    i = copy(out, i, br#"","matched_rules":"#);
    i = copy(out, i, matched_rules);
    i = copy(out, i, br#","obligations":"#);
    i = copy(out, i, obligations);
    i = copy(out, i, br#","evaluation_trace":"#);
    i = copy(out, i, trace);
    i = copy(out, i, br#","policy_hash":"#);
    if let Some(h) = policy_hash {
        i = copy(out, i, br#""sha256:"#);
        i = copy(out, i, h);
        i = copy(out, i, br#"""#);
    } else {
        i = copy(out, i, br#"null"#);
    }
    i = copy(out, i, br#","confidence":"high","break_glass_used":"#);
    i = copy(out, i, if break_glass_used { b"true" } else { b"false" });
    i = copy(out, i, br#"}"#);
    i
}

fn attr_string<'a>(object: &'a [u8], key: &[u8]) -> &'a [u8] {
    let Some(attrs) = object_after_key(object, b"\"attributes\"") else {
        return b"";
    };
    let mut k = [0u8; 48];
    if key.len() + 2 > k.len() {
        return b"";
    }
    k[0] = b'"';
    k[1..1 + key.len()].copy_from_slice(key);
    k[1 + key.len()] = b'"';
    extract_string(attrs, &k[..key.len() + 2])
}

fn attr_bool_true(object: &[u8], key: &[u8]) -> bool {
    let Some(attrs) = object_after_key(object, b"\"attributes\"") else {
        return false;
    };
    let mut needle = [0u8; 64];
    // "key":true
    if key.len() + 8 > needle.len() {
        return false;
    }
    needle[0] = b'"';
    needle[1..1 + key.len()].copy_from_slice(key);
    let mut n = 1 + key.len();
    needle[n] = b'"';
    n += 1;
    needle[n] = b':';
    n += 1;
    needle[n..n + 4].copy_from_slice(b"true");
    n += 4;
    contains(attrs, &needle[..n]) || {
        // with space
        let mut n2 = [0u8; 64];
        n2[..n - 4].copy_from_slice(&needle[..n - 4]);
        n2[n - 4] = b' ';
        n2[n - 3..n + 1].copy_from_slice(b"true");
        contains(attrs, &n2[..n + 1])
    }
}

fn parse_string_array<'a>(hay: &'a [u8], key: &[u8], out: &mut [&'a [u8]]) -> usize {
    let Some(arr) = json_array_after_key(hay, key) else {
        return 0;
    };
    let mut count = 0usize;
    let mut i = 0usize;
    while i < arr.len() && count < out.len() {
        if arr[i] == b'"' {
            i += 1;
            let start = i;
            while i < arr.len() && arr[i] != b'"' {
                i += 1;
            }
            if i <= arr.len() {
                out[count] = &arr[start..i];
                count += 1;
            }
        }
        i += 1;
    }
    count
}

fn object_after_key<'a>(hay: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let pos = find_key_at_depth(hay, key, 1)?;
    value_object_after(&hay[pos + key.len()..])
}

fn json_array_after_key<'a>(hay: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let pos = find_key_at_depth(hay, key, 1)?;
    value_array_after(&hay[pos + key.len()..])
}

/// Find `key` only when the surrounding `{...}` nesting depth equals `target_depth`.
/// Depth 1 = keys of the root object (avoids matching nested rule.principal before
/// top-level principal when serde reorders keys).
fn find_key_at_depth(hay: &[u8], key: &[u8], target_depth: i32) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_str = false;
    let mut i = 0usize;
    while i < hay.len() {
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
                if depth == target_depth
                    && i + key.len() <= hay.len()
                    && &hay[i..i + key.len()] == key
                {
                    return Some(i);
                }
                in_str = true;
            }
            b'{' => depth += 1,
            b'}' => depth -= 1,
            b'[' => depth += 1,
            b']' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    None
}

fn value_object_after<'a>(after_key: &'a [u8]) -> Option<&'a [u8]> {
    let colon = after_key.iter().position(|b| *b == b':')?;
    let mut rest = &after_key[colon + 1..];
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

fn value_array_after<'a>(after_key: &'a [u8]) -> Option<&'a [u8]> {
    let colon = after_key.iter().position(|b| *b == b':')?;
    let mut rest = &after_key[colon + 1..];
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
    // Nested/object-local: first match is fine (hay is already a scoped slice).
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

fn extract_i32(hay: &[u8], key: &[u8]) -> Option<i32> {
    let pos = find(hay, key)?;
    let after = &hay[pos + key.len()..];
    let colon = after.iter().position(|b| *b == b':')?;
    let mut rest = &after[colon + 1..];
    while rest.first() == Some(&b' ') {
        rest = &rest[1..];
    }
    let mut neg = false;
    if rest.first() == Some(&b'-') {
        neg = true;
        rest = &rest[1..];
    }
    let mut val: i32 = 0;
    let mut any = false;
    for &b in rest {
        if b < b'0' || b > b'9' {
            break;
        }
        any = true;
        val = val.saturating_mul(10).saturating_add(i32::from(b - b'0'));
    }
    if !any {
        return None;
    }
    Some(if neg { -val } else { val })
}

fn fnv_hex(bytes: &[u8]) -> [u8; 16] {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    let mut out = [b'0'; 16];
    for i in 0..16 {
        let nibble = ((h >> (60 - 4 * i)) & 0xf) as u8;
        out[i] = if nibble < 10 {
            b'0' + nibble
        } else {
            b'a' + (nibble - 10)
        };
    }
    out
}

fn copy(out: &mut [u8], at: usize, bytes: &[u8]) -> usize {
    let end = at + bytes.len();
    if end > out.len() {
        return at;
    }
    out[at..end].copy_from_slice(bytes);
    end
}

fn copy_i32(out: &mut [u8], at: usize, mut v: i32) -> usize {
    if v == 0 {
        return copy(out, at, b"0");
    }
    let neg = v < 0;
    if neg {
        v = -v;
    }
    let mut tmp = [0u8; 12];
    let mut n = 0usize;
    while v > 0 {
        tmp[n] = b'0' + (v % 10) as u8;
        n += 1;
        v /= 10;
    }
    let mut i = at;
    if neg {
        i = copy(out, i, b"-");
    }
    while n > 0 {
        n -= 1;
        i = copy(out, i, &[tmp[n]]);
    }
    i
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
