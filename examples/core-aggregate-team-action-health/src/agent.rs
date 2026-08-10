//! core.aggregate-team-action-health — team action-item health snapshot.
//!
//! Aggregates open items into total_open, overdue_count, on_track_pct,
//! overloaded_owners (open_count >= 2), and top_pressure_items (top 2 by score).
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

const MAX_OWNERS: usize = 64;
const ID_MAX: usize = 64;

struct OwnerSlot {
    id: [u8; ID_MAX],
    len: usize,
    count: u32,
}

struct PressureSlot {
    id: [u8; ID_MAX],
    len: usize,
    score_millis: u32,
}

impl Copy for OwnerSlot {}

impl Clone for OwnerSlot {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for PressureSlot {}

impl Clone for PressureSlot {
    fn clone(&self) -> Self {
        *self
    }
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
    let reference_date = extract_string_at_depth(input, b"\"reference_date\"", 1);
    let items = array_after_key_at_depth(input, b"\"items\"", 1).unwrap_or(b"[]");
    let config = object_after_key_at_depth(input, b"\"aggregation_config\"");

    if reference_date.is_empty() || config.is_none() {
        return write_error(out, b"invalid_input");
    }
    let _config = config.unwrap_or(b"{}");

    let mut total_open = 0u32;
    let mut overdue_count = 0u32;
    let mut non_overdue = 0u32;
    let mut owners = [OwnerSlot {
        id: [0; ID_MAX],
        len: 0,
        count: 0,
    }; MAX_OWNERS];
    let mut owner_count = 0usize;
    let mut top = [
        PressureSlot {
            id: [0; ID_MAX],
            len: 0,
            score_millis: 0,
        },
        PressureSlot {
            id: [0; ID_MAX],
            len: 0,
            score_millis: 0,
        },
    ];

    let mut pos = 1usize;
    while pos < items.len() {
        pos += skip_ws_comma(&items[pos..]);
        if pos >= items.len() || items[pos] == b']' {
            break;
        }
        if items[pos] != b'{' {
            break;
        }
        let Some(end) = balanced_end(&items[pos..], b'{', b'}') else {
            break;
        };
        let item = &items[pos..=pos + end];
        pos += end + 1;

        let status = extract_string(item, b"\"status\"");
        if !is_open_status(status) {
            continue;
        }

        total_open += 1;

        let due = extract_string(item, b"\"due_date\"");
        let overdue = !due.is_empty() && due < reference_date;
        if overdue {
            overdue_count += 1;
        } else {
            non_overdue += 1;
        }

        let owner = extract_string(item, b"\"owner_id\"");
        if !owner.is_empty() {
            bump_owner(&mut owners, &mut owner_count, owner);
        }

        let id = extract_string(item, b"\"id\"");
        if !id.is_empty() {
            let score = parse_pressure_score(item);
            insert_top_pressure(&mut top, id, score);
        }
    }

    let mut trace = [0u8; 64];
    let mut t = 0usize;
    t = copy(&mut trace, t, b"[\"aggregated ");
    t = write_u32(&mut trace, t, total_open);
    t = copy(&mut trace, t, b" items\"]");

    let mut pct_buf = [0u8; 16];
    let pct_len = format_on_track_pct(non_overdue, total_open, &mut pct_buf);

    let mut i = 0usize;
    i = copy(out, i, b"{\"total_open\":");
    i = write_u32(out, i, total_open);
    i = copy(out, i, b",\"on_track_pct\":");
    i = copy(out, i, &pct_buf[..pct_len]);
    i = copy(out, i, b",\"overdue_count\":");
    i = write_u32(out, i, overdue_count);
    i = copy(out, i, b",\"overloaded_owners\":");
    i = write_overloaded_owners(out, i, &owners[..owner_count]);
    i = copy(out, i, b",\"top_pressure_items\":");
    i = write_top_pressure(out, i, &top);
    i = copy(out, i, b",\"reason_code\":\"ok\",\"evaluation_trace\":");
    i = copy(out, i, &trace[..t]);
    i = copy(out, i, b"}");
    i
}

fn write_error(out: &mut [u8], reason: &[u8]) -> usize {
    let mut i = 0usize;
    i = copy(out, i, b"{\"total_open\":0,\"on_track_pct\":0,\"overdue_count\":0,\"overloaded_owners\":[],\"top_pressure_items\":[],\"reason_code\":\"");
    i = copy(out, i, reason);
    i = copy(out, i, b"\",\"evaluation_trace\":[]}");
    i
}

fn is_open_status(s: &[u8]) -> bool {
    matches!(
        s,
        b"open" | b"in_progress" | b"blocked" | b"snoozed"
    )
}

fn bump_owner(owners: &mut [OwnerSlot], owner_count: &mut usize, owner: &[u8]) {
    if owner.len() > ID_MAX {
        return;
    }
    for slot in owners.iter_mut().take(*owner_count) {
        if slot.len == owner.len() && slot.id[..slot.len] == owner[..] {
            slot.count += 1;
            return;
        }
    }
    if *owner_count >= owners.len() {
        return;
    }
    let slot = &mut owners[*owner_count];
    slot.id[..owner.len()].copy_from_slice(owner);
    slot.len = owner.len();
    slot.count = 1;
    *owner_count += 1;
}

fn insert_top_pressure(top: &mut [PressureSlot; 2], id: &[u8], score_millis: u32) {
    if id.len() > ID_MAX {
        return;
    }
    if score_millis > top[0].score_millis
        || (score_millis == top[0].score_millis && id_lt(id, &top[0].id[..top[0].len]))
    {
        top[1] = PressureSlot {
            id: top[0].id,
            len: top[0].len,
            score_millis: top[0].score_millis,
        };
        top[0].id[..id.len()].copy_from_slice(id);
        top[0].len = id.len();
        top[0].score_millis = score_millis;
        return;
    }
    if score_millis > top[1].score_millis
        || (score_millis == top[1].score_millis && id_lt(id, &top[1].id[..top[1].len]))
    {
        top[1].id[..id.len()].copy_from_slice(id);
        top[1].len = id.len();
        top[1].score_millis = score_millis;
    }
}

fn id_lt(a: &[u8], b: &[u8]) -> bool {
    let n = if a.len() < b.len() { a.len() } else { b.len() };
    if a[..n] != b[..n] {
        return a[..n] < b[..n];
    }
    a.len() < b.len()
}

fn format_on_track_pct(non_overdue: u32, total_open: u32, out: &mut [u8]) -> usize {
    if total_open == 0 {
        return copy(out, 0, b"0");
    }
    let tenths = non_overdue * 1000 / total_open;
    let whole = tenths / 10;
    let frac = tenths % 10;
    let mut i = write_u32(out, 0, whole);
    i = copy(out, i, b".");
    out[i] = b'0' + frac as u8;
    i + 1
}

fn write_overloaded_owners(out: &mut [u8], mut i: usize, owners: &[OwnerSlot]) -> usize {
    i = copy(out, i, b"[");
    let mut first = true;
    for slot in owners {
        if slot.count < 2 {
            continue;
        }
        if !first {
            i = copy(out, i, b",");
        }
        first = false;
        i = copy(out, i, b"{\"owner_id\":\"");
        i = copy(out, i, &slot.id[..slot.len]);
        i = copy(out, i, b"\",\"open_count\":");
        i = write_u32(out, i, slot.count);
        i = copy(out, i, b"}");
    }
    copy(out, i, b"]")
}

fn write_top_pressure(out: &mut [u8], mut i: usize, top: &[PressureSlot; 2]) -> usize {
    i = copy(out, i, b"[");
    let mut wrote = 0usize;
    for slot in top {
        if slot.len == 0 {
            continue;
        }
        if wrote > 0 {
            i = copy(out, i, b",");
        }
        i = copy(out, i, b"\"");
        i = copy(out, i, &slot.id[..slot.len]);
        i = copy(out, i, b"\"");
        wrote += 1;
    }
    copy(out, i, b"]")
}

fn parse_pressure_score(obj: &[u8]) -> u32 {
    let Some(pos) = find(obj, b"\"pressure_score\"") else {
        return 0;
    };
    let after = &obj[pos + b"\"pressure_score\"".len()..];
    let Some(colon) = after.iter().position(|b| *b == b':') else {
        return 0;
    };
    let rest = skip_ws(&after[colon + 1..]);
    if rest.is_empty() {
        return 0;
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
    whole * 1000 + frac
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

fn object_after_key_at_depth<'a>(hay: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
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

fn extract_string_at_depth<'a>(hay: &'a [u8], key: &[u8], depth: i32) -> &'a [u8] {
    let Some(pos) = find_key_at_depth(hay, key, depth) else {
        return b"";
    };
    string_value_after(&hay[pos + key.len()..])
}

fn extract_string<'a>(hay: &'a [u8], key: &[u8]) -> &'a [u8] {
    let Some(pos) = find(hay, key) else {
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
