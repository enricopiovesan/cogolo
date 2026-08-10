//! core.select-items-for-followup — deterministic follow-up item selection.
//!
//! Respects quiet hours, snooze, per-user nudge budgets, and escalation thresholds.
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
static mut OUTPUT_BUF: [u8; 16384] = [0; 16384];

const MAX_ITEMS: usize = 16;

#[derive(Clone, Copy)]
struct Selected {
    item_id: [u8; 48],
    item_id_len: usize,
    intensity: [u8; 12],
    intensity_len: usize,
    reason: [u8; 128],
    reason_len: usize,
    channel: [u8; 24],
    channel_len: usize,
}

#[derive(Clone, Copy)]
struct Skipped {
    item_id: [u8; 48],
    item_id_len: usize,
    reason: [u8; 24],
    reason_len: usize,
    detail: [u8; 128],
    detail_len: usize,
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
    let ref_dt = extract_string_at_depth(input, b"\"reference_datetime\"", 1);
    let config = object_after_key(input, b"\"followup_config\"").unwrap_or(b"");
    let prefs = object_after_key(input, b"\"user_preferences\"").unwrap_or(b"");
    let Some(open_items) = array_after_key_at_depth(input, b"\"open_items\"", 1) else {
        return fail(out, b"config_error", br#"["open_items missing"]"#);
    };
    if ref_dt.is_empty() || config.is_empty() {
        return fail(out, b"config_error", br#"["required fields missing"]"#);
    }

    let respect_quiet = extract_bool(config, b"\"respect_quiet_hours\"").unwrap_or(true);
    let escalate_after = extract_i32(config, b"\"escalate_after_nudges\"").unwrap_or(3);
    let min_pressure = extract_number_scaled(config, b"\"min_pressure_for_soft\"").unwrap_or(400);
    let soft_days = extract_i32(config, b"\"soft_days_before_due\"").unwrap_or(2);

    let ref_date = &ref_dt[..ref_dt.len().min(10)];

    let mut selected: [Selected; MAX_ITEMS] = [Selected {
        item_id: [0; 48],
        item_id_len: 0,
        intensity: [0; 12],
        intensity_len: 0,
        reason: [0; 128],
        reason_len: 0,
        channel: [0; 24],
        channel_len: 0,
    }; MAX_ITEMS];
    let mut skipped: [Skipped; MAX_ITEMS] = [Skipped {
        item_id: [0; 48],
        item_id_len: 0,
        reason: [0; 24],
        reason_len: 0,
        detail: [0; 128],
        detail_len: 0,
    }; MAX_ITEMS];
    let mut sel_count = 0usize;
    let mut skip_count = 0usize;
    let mut item_count = 0usize;
    let mut any_quiet = false;
    let mut any_escalate = false;

    for_each_object_in_array(open_items, |item| {
        if item_count >= MAX_ITEMS {
            return;
        }
        item_count += 1;

        let item_id = extract_string(item, b"\"id\"");
        let owner = extract_string(item, b"\"owner_id\"");
        let due = extract_string(item, b"\"due_date\"");
        let snoozed = extract_optional_string(item, b"\"snoozed_until\"");
        let nudge_count = extract_i32(item, b"\"nudge_count\"").unwrap_or(0);
        let pressure = extract_number_scaled(item, b"\"pressure_score\"").unwrap_or(0);
        let last_activity = extract_string(item, b"\"last_activity_at\"");

        if !snoozed.is_empty() && snoozed > ref_dt {
            push_skipped(
                &mut skipped,
                &mut skip_count,
                item_id,
                b"snoozed",
                b"item is snoozed until later",
            );
            return;
        }

        if respect_quiet {
            if let Some(owner_prefs) = user_pref_object(prefs, owner) {
                if let Some(qh) = object_after_key(owner_prefs, b"\"quiet_hours\"") {
                    if in_quiet_hours(ref_dt, qh) {
                        any_quiet = true;
                        let mut detail = [0u8; 96];
                        let mut d = 0usize;
                        d = copy(&mut detail, d, b"owner is inside quiet hours ");
                        let start = extract_string(qh, b"\"start\"");
                        let end = extract_string(qh, b"\"end\"");
                        d = copy(&mut detail, d, start);
                        d = copy(&mut detail, d, b"-");
                        d = copy(&mut detail, d, end);
                        push_skipped_detail(
                            &mut skipped,
                            &mut skip_count,
                            item_id,
                            b"quiet_hours",
                            &detail[..d],
                        );
                        return;
                    }
                }
            }
        }

        let max_nudges = user_pref_i32(prefs, owner, b"\"max_nudges_per_day\"").unwrap_or(2);
        let sent_today =
            user_pref_i32(prefs, owner, b"\"nudges_sent_today\"").unwrap_or(0);
        if sent_today >= max_nudges {
            push_skipped(
                &mut skipped,
                &mut skip_count,
                item_id,
                b"budget_exceeded",
                b"daily nudge budget exhausted",
            );
            return;
        }

        if pressure < min_pressure {
            push_skipped(
                &mut skipped,
                &mut skip_count,
                item_id,
                b"low_pressure",
                b"pressure below soft threshold",
            );
            return;
        }

        let overdue = !due.is_empty() && due < ref_date;

        if overdue && nudge_count >= escalate_after {
            any_escalate = true;
            push_selected(
                &mut selected,
                &mut sel_count,
                item_id,
                b"escalate",
                b"3 prior nudges ignored + overdue + high pressure",
                b"manager",
            );
            return;
        }

        if overdue {
            push_selected(
                &mut selected,
                &mut sel_count,
                item_id,
                b"direct",
                b"item is overdue",
                b"",
            );
            return;
        }

        if approaching_due(due, ref_date, soft_days) || pressure >= min_pressure {
            push_selected(
                &mut selected,
                &mut sel_count,
                item_id,
                b"soft",
                b"approaching due date or sufficient pressure",
                b"",
            );
            return;
        }

        if !last_activity.is_empty() {
            let act_date = &last_activity[..last_activity.len().min(10)];
            if act_date >= ref_date {
                push_skipped(
                    &mut skipped,
                    &mut skip_count,
                    item_id,
                    b"recently_active",
                    b"owner recently active on item",
                );
                return;
            }
        }

        push_skipped(
            &mut skipped,
            &mut skip_count,
            item_id,
            b"low_pressure",
            b"no follow-up signal",
        );
    });

    let mut trace = [0u8; 512];
    let mut t = 0usize;
    t = copy(&mut trace, t, b"[\"");
    t = write_usize(&mut trace, t, item_count);
    t = copy(&mut trace, t, b" item evaluated");
    if item_count != 1 {
        t = copy(&mut trace, t, b"s");
    }
    t = copy(&mut trace, t, b"\"");
    if any_quiet && sel_count == 0 && skip_count > 0 {
        t = copy(&mut trace, t, b", \"skipped due to quiet_hours\"");
    }
    if any_escalate {
        t = copy(&mut trace, t, b", \"escalation threshold met\"");
    }
    t = copy(&mut trace, t, b"]");

    write_output(out, &selected[..sel_count], &skipped[..skip_count], &trace[..t])
}

fn write_output(
    out: &mut [u8],
    selected: &[Selected],
    skipped: &[Skipped],
    trace: &[u8],
) -> usize {
    let mut i = 0usize;
    i = copy(out, i, b"{\"selected\":[");
    for (idx, s) in selected.iter().enumerate() {
        if idx > 0 {
            i = copy(out, i, b",");
        }
        i = copy(out, i, b"{\"item_id\":\"");
        i = copy(out, i, &s.item_id[..s.item_id_len]);
        i = copy(out, i, b"\",\"intensity\":\"");
        i = copy(out, i, &s.intensity[..s.intensity_len]);
        i = copy(out, i, b"\",\"reason\":\"");
        i = copy_json_escaped(out, i, &s.reason[..s.reason_len]);
        if s.channel_len > 0 {
            i = copy(out, i, b"\",\"recommended_channel\":\"");
            i = copy(out, i, &s.channel[..s.channel_len]);
        }
        i = copy(out, i, b"\"}");
    }
    i = copy(out, i, b"],\"skipped\":[");
    for (idx, s) in skipped.iter().enumerate() {
        if idx > 0 {
            i = copy(out, i, b",");
        }
        i = copy(out, i, b"{\"item_id\":\"");
        i = copy(out, i, &s.item_id[..s.item_id_len]);
        i = copy(out, i, b"\",\"reason\":\"");
        i = copy(out, i, &s.reason[..s.reason_len]);
        i = copy(out, i, b"\",\"detail\":\"");
        i = copy_json_escaped(out, i, &s.detail[..s.detail_len]);
        i = copy(out, i, b"\"}");
    }
    i = copy(out, i, b"],\"reason_code\":\"ok\",\"evaluation_trace\":");
    i = copy(out, i, trace);
    i = copy(out, i, b"}");
    i
}

fn fail(out: &mut [u8], code: &[u8], trace: &[u8]) -> usize {
    let mut i = 0usize;
    i = copy(out, i, b"{\"selected\":[],\"skipped\":[],\"reason_code\":\"");
    i = copy(out, i, code);
    i = copy(out, i, b"\",\"evaluation_trace\":");
    i = copy(out, i, trace);
    i = copy(out, i, b"}");
    i
}

fn push_selected(
    selected: &mut [Selected],
    count: &mut usize,
    item_id: &[u8],
    intensity: &[u8],
    reason: &[u8],
    channel: &[u8],
) {
    if *count >= MAX_ITEMS {
        return;
    }
    let mut s = Selected {
        item_id: [0; 48],
        item_id_len: 0,
        intensity: [0; 12],
        intensity_len: 0,
        reason: [0; 128],
        reason_len: 0,
        channel: [0; 24],
        channel_len: 0,
    };
    s.item_id_len = copy(&mut s.item_id, 0, item_id);
    s.intensity_len = copy(&mut s.intensity, 0, intensity);
    s.reason_len = copy(&mut s.reason, 0, reason);
    if !channel.is_empty() {
        s.channel_len = copy(&mut s.channel, 0, channel);
    }
    selected[*count] = s;
    *count += 1;
}

fn push_skipped(
    skipped: &mut [Skipped],
    count: &mut usize,
    item_id: &[u8],
    reason: &[u8],
    detail: &[u8],
) {
    push_skipped_detail(skipped, count, item_id, reason, detail);
}

fn push_skipped_detail(
    skipped: &mut [Skipped],
    count: &mut usize,
    item_id: &[u8],
    reason: &[u8],
    detail: &[u8],
) {
    if *count >= MAX_ITEMS {
        return;
    }
    let mut s = Skipped {
        item_id: [0; 48],
        item_id_len: 0,
        reason: [0; 24],
        reason_len: 0,
        detail: [0; 128],
        detail_len: 0,
    };
    s.item_id_len = copy(&mut s.item_id, 0, item_id);
    s.reason_len = copy(&mut s.reason, 0, reason);
    s.detail_len = copy(&mut s.detail, 0, detail);
    skipped[*count] = s;
    *count += 1;
}

fn for_each_object_in_array(arr: &[u8], mut f: impl FnMut(&[u8])) {
    if arr.first() != Some(&b'[') {
        return;
    }
    let mut i = 1usize;
    while i < arr.len() {
        while i < arr.len()
            && (arr[i] == b' '
                || arr[i] == b'\n'
                || arr[i] == b'\t'
                || arr[i] == b'\r'
                || arr[i] == b',')
        {
            i += 1;
        }
        if i >= arr.len() || arr[i] == b']' {
            break;
        }
        if arr[i] == b'{' {
            if let Some(end) = balanced_end(&arr[i..], b'{', b'}') {
                f(&arr[i..=i + end]);
                i += end + 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }
}

fn user_pref_object<'a>(prefs: &'a [u8], owner: &[u8]) -> Option<&'a [u8]> {
    if prefs.is_empty() || owner.is_empty() {
        return None;
    }
    let mut key = [0u8; 64];
    let mut k = 0usize;
    k = copy(&mut key, k, b"\"");
    k = copy(&mut key, k, owner);
    k = copy(&mut key, k, b"\"");
    object_after_key(prefs, &key[..k])
}

fn user_pref_i32(prefs: &[u8], owner: &[u8], key: &[u8]) -> Option<i32> {
    let obj = user_pref_object(prefs, owner)?;
    extract_i32(obj, key)
}

fn in_quiet_hours(ref_dt: &[u8], qh: &[u8]) -> bool {
    let hour = parse_ref_hour(ref_dt);
    let start = parse_time_hour(extract_string(qh, b"\"start\""));
    let end = parse_time_hour(extract_string(qh, b"\"end\""));
    if start <= end {
        hour >= start && hour < end
    } else {
        hour >= start || hour < end
    }
}

fn parse_ref_hour(ref_dt: &[u8]) -> i32 {
    let Some(t_pos) = ref_dt.iter().position(|b| *b == b'T') else {
        return 0;
    };
    let after = &ref_dt[t_pos + 1..];
    parse_two_digit_hour(after)
}

fn parse_time_hour(time: &[u8]) -> i32 {
    parse_two_digit_hour(time)
}

fn parse_two_digit_hour(s: &[u8]) -> i32 {
    if s.len() < 2 {
        return 0;
    }
    let h0 = s[0];
    let h1 = s[1];
    if h0 < b'0' || h0 > b'9' || h1 < b'0' || h1 > b'9' {
        return 0;
    }
    i32::from(h0 - b'0') * 10 + i32::from(h1 - b'0')
}

fn approaching_due(due: &[u8], ref_date: &[u8], soft_days: i32) -> bool {
    if due.is_empty() || ref_date.is_empty() {
        return false;
    }
    if due == ref_date {
        return true;
    }
    if due > ref_date {
        return days_between(ref_date, due) <= soft_days;
    }
    false
}

fn days_between(from: &[u8], to: &[u8]) -> i32 {
    if from.len() < 10 || to.len() < 10 {
        return 999;
    }
    let fy = parse_year(from);
    let fm = parse_month(from);
    let fd = parse_day(from);
    let ty = parse_year(to);
    let tm = parse_month(to);
    let td = parse_day(to);
    let from_days = fy * 372 + fm * 31 + fd;
    let to_days = ty * 372 + tm * 31 + td;
    to_days - from_days
}

fn parse_year(d: &[u8]) -> i32 {
    parse_digits(&d[..4.min(d.len())])
}

fn parse_month(d: &[u8]) -> i32 {
    if d.len() < 7 {
        return 0;
    }
    parse_digits(&d[5..7])
}

fn parse_day(d: &[u8]) -> i32 {
    if d.len() < 10 {
        return 0;
    }
    parse_digits(&d[8..10])
}

fn parse_digits(s: &[u8]) -> i32 {
    let mut val = 0i32;
    for &b in s {
        if b < b'0' || b > b'9' {
            break;
        }
        val = val.saturating_mul(10).saturating_add(i32::from(b - b'0'));
    }
    val
}

fn write_usize(out: &mut [u8], at: usize, n: usize) -> usize {
    if n == 0 {
        return copy(out, at, b"0");
    }
    let mut buf = [0u8; 20];
    let mut v = n;
    let mut len = 0usize;
    while v > 0 {
        buf[len] = b'0' + (v % 10) as u8;
        len += 1;
        v /= 10;
    }
    let mut i = at;
    while len > 0 {
        len -= 1;
        if i < out.len() {
            out[i] = buf[len];
            i += 1;
        }
    }
    i
}

fn extract_optional_string<'a>(hay: &'a [u8], key: &[u8]) -> &'a [u8] {
    let Some(pos) = find(hay, key) else {
        return b"";
    };
    let after = &hay[pos + key.len()..];
    let Some(colon) = after.iter().position(|b| *b == b':') else {
        return b"";
    };
    let rest = skip_ws(&after[colon + 1..]);
    if rest.starts_with(b"null") {
        return b"";
    }
    if rest.first() != Some(&b'"') {
        return b"";
    }
    let inner = &rest[1..];
    let Some(end) = inner.iter().position(|b| *b == b'"') else {
        return b"";
    };
    &inner[..end]
}

fn extract_number_scaled(hay: &[u8], key: &[u8]) -> Option<i32> {
    let pos = find(hay, key)?;
    let after = &hay[pos + key.len()..];
    let colon = after.iter().position(|b| *b == b':')?;
    let mut rest = skip_ws(&after[colon + 1..]);
    let mut neg = false;
    if rest.first() == Some(&b'-') {
        neg = true;
        rest = &rest[1..];
    }
    let mut whole = 0i32;
    let mut frac = 0i32;
    let mut frac_digits = 0i32;
    let mut in_frac = false;
    let mut any = false;
    for &b in rest {
        if b == b'.' {
            in_frac = true;
            continue;
        }
        if b < b'0' || b > b'9' {
            break;
        }
        any = true;
        if in_frac {
            if frac_digits < 3 {
                frac = frac.saturating_mul(10).saturating_add(i32::from(b - b'0'));
                frac_digits += 1;
            }
        } else {
            whole = whole
                .saturating_mul(10)
                .saturating_add(i32::from(b - b'0'));
        }
    }
    if !any {
        return None;
    }
    while frac_digits < 3 {
        frac = frac.saturating_mul(10);
        frac_digits += 1;
    }
    let scaled = whole.saturating_mul(1000).saturating_add(frac);
    Some(if neg { -scaled } else { scaled })
}

fn array_after_key_at_depth<'a>(hay: &'a [u8], key: &[u8], depth: i32) -> Option<&'a [u8]> {
    let pos = find_key_at_depth(hay, key, depth)?;
    let after = &hay[pos + key.len()..];
    let colon = after.iter().position(|b| *b == b':')?;
    let rest = skip_ws(&after[colon + 1..]);
    if rest.first() != Some(&b'[') {
        return None;
    }
    let end = balanced_end(rest, b'[', b']')?;
    Some(&rest[..=end])
}

fn object_after_key<'a>(hay: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let pos = find(hay, key)?;
    let after = &hay[pos + key.len()..];
    let colon = after.iter().position(|b| *b == b':')?;
    let rest = skip_ws(&after[colon + 1..]);
    if rest.first() != Some(&b'{') {
        return None;
    }
    let end = balanced_end(rest, b'{', b'}')?;
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

fn extract_i32(hay: &[u8], key: &[u8]) -> Option<i32> {
    let pos = find(hay, key)?;
    let after = &hay[pos + key.len()..];
    let colon = after.iter().position(|b| *b == b':')?;
    let mut rest = skip_ws(&after[colon + 1..]);
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

fn copy_json_escaped(out: &mut [u8], mut i: usize, s: &[u8]) -> usize {
    for &b in s {
        match b {
            b'"' => i = copy(out, i, b"\\\""),
            b'\\' => i = copy(out, i, b"\\\\"),
            _ => {
                if i < out.len() {
                    out[i] = b;
                    i += 1;
                }
            }
        }
    }
    i
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
