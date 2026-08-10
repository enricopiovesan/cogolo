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
    let item = object_after_key_at_depth(input, b"\"item\"", 1);
    let reference_date = extract_string_at_depth(input, b"\"reference_date\"", 1);
    let config = object_after_key_at_depth(input, b"\"pressure_config\"", 1);
    if item.is_none() || reference_date.is_empty() || config.is_none() {
        return fail(out, b"config_error", b"", 0, b"unknown");
    }
    let item = item.unwrap_or(b"{}");
    let config = config.unwrap_or(b"{}");
    let item_id = extract_string(item, b"\"id\"");
    let due = extract_string(item, b"\"due_date\"");
    if due.is_empty() {
        return fail(out, b"missing_due_date", item_id, 0, b"unknown");
    }
    let Some(due_days) = parse_ymd_days(due) else {
        return fail(out, b"invalid_date", item_id, 0, b"unknown");
    };
    let Some(ref_days) = parse_ymd_days(reference_date) else {
        return fail(out, b"invalid_date", item_id, 0, b"unknown");
    };
    let horizon = extract_i32(config, b"\"horizon_days\"").unwrap_or(14);
    if horizon <= 0 {
        return fail(out, b"config_error", item_id, 0, b"unknown");
    }
    let days_until = due_days - ref_days;
    let (band, score_millis): (&[u8], u32) = if days_until < 0 {
        (b"overdue" as &[u8], 1000)
    } else if days_until > horizon {
        (b"low" as &[u8], 0)
    } else {
        // score = 1 - days/horizon  in millis
        let score = 1000u32.saturating_sub((days_until as u32 * 1000) / horizon as u32);
        let band: &[u8] = if days_until <= 2 {
            b"high"
        } else if days_until <= 7 {
            b"medium"
        } else {
            b"low"
        };
        (band, score)
    };

    let mut score_buf = [0u8; 16];
    let score_len = format_score_millis(&mut score_buf, score_millis);

    let mut i = 0usize;
    i = copy(out, i, b"{\"item_id\":\"");
    i = copy_json_escaped(out, i, item_id);
    i = copy(out, i, b"\",\"pressure_score\":");
    i = copy(out, i, &score_buf[..score_len]);
    i = copy(out, i, b",\"days_until_due\":");
    i = write_i32(out, i, days_until);
    i = copy(out, i, b",\"pressure_band\":\"");
    i = copy(out, i, band);
    i = copy(out, i, b"\",\"reason_code\":\"ok\",\"evaluation_trace\":[\"days_until_due=");
    i = write_i32(out, i, days_until);
    i = copy(out, i, b"\",\"horizon=");
    i = write_i32(out, i, horizon);
    i = copy(out, i, b"\",\"band=");
    i = copy(out, i, band);
    i = copy(out, i, b"\"]}");
    i
}

fn fail(out: &mut [u8], code: &[u8], item_id: &[u8], days: i32, band: &[u8]) -> usize {
    let mut i = 0usize;
    i = copy(out, i, b"{\"item_id\":\"");
    i = copy_json_escaped(out, i, item_id);
    i = copy(out, i, b"\",\"pressure_score\":0,\"days_until_due\":");
    i = write_i32(out, i, days);
    i = copy(out, i, b",\"pressure_band\":\"");
    i = copy(out, i, band);
    i = copy(out, i, b"\",\"reason_code\":\"");
    i = copy(out, i, code);
    i = copy(out, i, b"\",\"evaluation_trace\":[]}");
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

fn skip_ws_comma(s: &[u8]) -> usize {
    let mut i = 0usize;
    while i < s.len() {
        match s[i] {
            b' ' | b'\n' | b'\t' | b'\r' | b',' => i += 1,
            _ => break,
        }
    }
    i
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

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
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
    let rest = skip_ws(&after_key[colon + 1..]);
    if rest.first() != Some(&b'"') {
        return b"";
    }
    let rest = &rest[1..];
    let Some(end) = rest.iter().position(|b| *b == b'"') else {
        return b"";
    };
    &rest[..end]
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

fn object_after_key_at_depth<'a>(hay: &'a [u8], key: &[u8], depth: i32) -> Option<&'a [u8]> {
    let pos = find_key_at_depth(hay, key, depth)?;
    let after = &hay[pos + key.len()..];
    let colon = after.iter().position(|b| *b == b':')?;
    let rest = skip_ws(&after[colon + 1..]);
    if rest.first() != Some(&b'{') {
        return None;
    }
    let end = balanced_end(rest, b'{', b'}')?;
    Some(&rest[..=end])
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
    let rest = skip_ws(&after[colon + 1..]);
    parse_i32(rest)
}

fn parse_i32(rest: &[u8]) -> Option<i32> {
    if rest.is_empty() {
        return None;
    }
    let mut neg = false;
    let mut j = 0usize;
    if rest[0] == b'-' {
        neg = true;
        j = 1;
    }
    if j >= rest.len() || rest[j] < b'0' || rest[j] > b'9' {
        return None;
    }
    let mut n: i32 = 0;
    while j < rest.len() && rest[j] >= b'0' && rest[j] <= b'9' {
        n = n * 10 + (rest[j] - b'0') as i32;
        j += 1;
    }
    Some(if neg { -n } else { n })
}

fn parse_number_millis(hay: &[u8], key: &[u8]) -> Option<u32> {
    let pos = find(hay, key)?;
    let after = &hay[pos + key.len()..];
    let colon = after.iter().position(|b| *b == b':')?;
    let rest = skip_ws(&after[colon + 1..]);
    if rest.is_empty() {
        return None;
    }
    let mut whole: u32 = 0;
    let mut frac: u32 = 0;
    let mut frac_digits = 0u32;
    let mut seen_dot = false;
    let mut j = 0usize;
    while j < rest.len() {
        let b = rest[j];
        if b == b',' || b == b'}' || b == b']' || b == b' ' || b == b'\n' {
            break;
        }
        if b == b'.' {
            seen_dot = true;
            j += 1;
            continue;
        }
        if b < b'0' || b > b'9' {
            break;
        }
        let digit = (b - b'0') as u32;
        if seen_dot {
            if frac_digits < 3 {
                frac = frac * 10 + digit;
                frac_digits += 1;
            }
        } else {
            whole = whole * 10 + digit;
        }
        j += 1;
    }
    while frac_digits < 3 {
        frac *= 10;
        frac_digits += 1;
    }
    Some(whole * 1000 + frac)
}

fn copy(out: &mut [u8], at: usize, bytes: &[u8]) -> usize {
    let end = at + bytes.len();
    if end > out.len() {
        return at;
    }
    out[at..end].copy_from_slice(bytes);
    end
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

fn write_u32(out: &mut [u8], mut i: usize, mut n: u32) -> usize {
    if n == 0 {
        if i < out.len() {
            out[i] = b'0';
            return i + 1;
        }
        return i;
    }
    let mut digits = [0u8; 10];
    let mut d = 0usize;
    while n > 0 {
        digits[d] = b'0' + (n % 10) as u8;
        n /= 10;
        d += 1;
    }
    while d > 0 {
        d -= 1;
        if i < out.len() {
            out[i] = digits[d];
            i += 1;
        }
    }
    i
}

fn write_i32(out: &mut [u8], mut i: usize, n: i32) -> usize {
    if n < 0 {
        i = copy(out, i, b"-");
        write_u32(out, i, (-n) as u32)
    } else {
        write_u32(out, i, n as u32)
    }
}

fn ascii_lower(b: u8) -> u8 {
    if b >= b'A' && b <= b'Z' {
        b + 32
    } else {
        b
    }
}

fn eq_ignore_case(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    for i in 0..a.len() {
        if ascii_lower(a[i]) != ascii_lower(b[i]) {
            return false;
        }
    }
    true
}

fn normalize_email(src: &[u8], dst: &mut [u8]) -> usize {
    let mut i = 0usize;
    let mut j = 0usize;
    while i < src.len() && (src[i] == b' ' || src[i] == b'\t') {
        i += 1;
    }
    let mut end = src.len();
    while end > i && (src[end - 1] == b' ' || src[end - 1] == b'\t') {
        end -= 1;
    }
    while i < end && j < dst.len() {
        dst[j] = ascii_lower(src[i]);
        i += 1;
        j += 1;
    }
    j
}

fn trim_ascii(s: &[u8]) -> &[u8] {
    let mut start = 0usize;
    let mut end = s.len();
    while start < end && matches!(s[start], b' ' | b'\t' | b'\n' | b'\r') {
        start += 1;
    }
    while end > start && matches!(s[end - 1], b' ' | b'\t' | b'\n' | b'\r') {
        end -= 1;
    }
    &s[start..end]
}

/// Days since 1970-01-01 for YYYY-MM-DD (Howard Hinnant civil_from_days inverse).
fn parse_ymd_days(s: &[u8]) -> Option<i32> {
    if s.len() < 10 || s[4] != b'-' || s[7] != b'-' {
        return None;
    }
    let y = parse_i32(&s[0..4])?;
    let m = parse_i32(&s[5..7])?;
    let d = parse_i32(&s[8..10])?;
    if m < 1 || m > 12 || d < 1 || d > 31 {
        return None;
    }
    let y = y as i32 - if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let mp = if m > 2 { (m - 3) as u32 } else { (m + 9) as u32 };
    let doy = (153 * mp + 2) / 5 + d as u32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some((era * 146097 + doe as i32) - 719468)
}

fn format_score_millis(out: &mut [u8], millis: u32) -> usize {
    let whole = millis / 1000;
    let frac = millis % 1000;
    let mut i = write_u32(out, 0, whole);
    i = copy(out, i, b".");
    // always 3 digits for determinism in contract examples we may trim; write without trailing zeros carefully
    // Use up to 3 digits, trim trailing zeros but keep at least one if frac!=0? Contract examples use 0.785 / 1.0
    if frac == 0 {
        i = copy(out, i, b"0");
        return i;
    }
    let d0 = (frac / 100) as u8;
    let d1 = ((frac / 10) % 10) as u8;
    let d2 = (frac % 10) as u8;
    out[i] = b'0' + d0;
    i += 1;
    if d1 != 0 || d2 != 0 {
        out[i] = b'0' + d1;
        i += 1;
        if d2 != 0 {
            out[i] = b'0' + d2;
            i += 1;
        }
    }
    i
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
