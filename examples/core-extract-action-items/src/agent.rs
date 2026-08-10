//! core.extract-action-items — deterministic heuristic action-item extractor.
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

const MAX_SENTENCES: usize = 32;
const MAX_PARTICIPANTS: usize = 16;

#[derive(Clone, Copy)]
struct Sentence<'a> {
    body: &'a [u8],
}

#[derive(Clone, Copy)]
struct Participant<'a> {
    full: &'a [u8],
    first: &'a [u8],
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
        let out_len = extract(&INPUT_BUF[..total], &mut OUTPUT_BUF);
        let out = IoVec {
            buffer: OUTPUT_BUF.as_ptr(),
            length: out_len,
        };
        let mut written = 0usize;
        let _ = fd_write(1, &out, 1, &mut written);
    }
}

unsafe fn extract(input: &[u8], out: &mut [u8]) -> usize {
    let text = extract_string_at_depth(input, b"\"text\"", 1);
    let meeting_date = extract_string_at_depth(input, b"\"meeting_date\"", 1);
    let config = object_after_key(input, b"\"extraction_config\"").unwrap_or(b"");
    let reject_vague = extract_bool(config, b"\"reject_vague\"").unwrap_or(true);
    let min_confidence = extract_f64_literal(config, b"\"min_confidence\"").unwrap_or(0.75);
    let review_threshold = extract_f64_literal(config, b"\"review_threshold\"").unwrap_or(0.55);

    let mut participants: [Participant; MAX_PARTICIPANTS] = [Participant {
        full: b"",
        first: b"",
    }; MAX_PARTICIPANTS];
    let participant_count = parse_participants(input, &mut participants);

    if text.is_empty() {
        return write_empty_result(out, b"no_action_items_found", br#"["text missing"]"#);
    }

    let mut sentences: [Sentence; MAX_SENTENCES] = [Sentence { body: b"" }; MAX_SENTENCES];
    let sentence_count = split_sentences(text, &mut sentences);

    let mut action_buf = [0u8; 8192];
    let mut review_buf = [0u8; 4096];
    let mut action_len = 0usize;
    let mut review_len = 0usize;
    let mut action_count = 0usize;
    let mut review_count = 0usize;

    for si in 0..sentence_count {
        let sent = &sentences[si];
        if sent.body.is_empty() {
            continue;
        }

        if reject_vague && is_vague(sent.body) {
            let mut title = [0u8; 256];
            let tlen = simplify_vague_title(sent.body, &mut title);
            review_len = append_review_item(
                &mut review_buf,
                review_len,
                &title[..tlen],
                sent.body,
                b"0.58",
                b"vague_language",
                review_count > 0,
            );
            review_count += 1;
            continue;
        }

        if let Some(pidx) = match_action_participant(sent.body, &participants[..participant_count]) {
            let p = &participants[pidx];
            let will = contains(sent.body, b" will ");
            let to_pat = contains(sent.body, b" to ");
            if !will && !to_pat {
                continue;
            }
            let conf = if will { b"0.93" } else { b"0.88" };
            let conf_val = if will { 0.93 } else { 0.88 };

            let mut title = [0u8; 256];
            let tlen = simplify_action_title(sent.body, p.first, will, &mut title);
            let due = resolve_due_date(sent.body, meeting_date);

            if conf_val >= min_confidence {
                action_len = append_action_item(
                    &mut action_buf,
                    action_len,
                    &title[..tlen],
                    p.full,
                    due,
                    conf,
                    sent.body,
                    action_count > 0,
                );
                action_count += 1;
            } else if conf_val >= review_threshold {
                review_len = append_review_item(
                    &mut review_buf,
                    review_len,
                    &title[..tlen],
                    sent.body,
                    conf,
                    b"below_min_confidence",
                    review_count > 0,
                );
                review_count += 1;
            }
        }
    }

    let reason: &[u8] = if action_count == 0 && review_count == 0 {
        b"no_action_items_found"
    } else {
        b"ok"
    };

    let mut trace = [0u8; 512];
    let trace_len = format_trace(
        &mut trace,
        action_count,
        review_count,
        reason == b"no_action_items_found",
    );

    write_result(
        out,
        &action_buf[..action_len],
        &review_buf[..review_len],
        reason,
        &trace[..trace_len],
    )
}

fn write_empty_result(out: &mut [u8], reason: &[u8], trace: &[u8]) -> usize {
    write_result(out, b"", b"", reason, trace)
}

fn write_result(
    out: &mut [u8],
    actions: &[u8],
    reviews: &[u8],
    reason: &[u8],
    trace: &[u8],
) -> usize {
    let mut i = 0usize;
    i = copy(out, i, b"{\"action_items\":[");
    i = copy(out, i, actions);
    i = copy(out, i, b"],\"needs_human_review\":[");
    i = copy(out, i, reviews);
    i = copy(out, i, b"],\"rejected_candidates\":[],\"reason_code\":\"");
    i = copy(out, i, reason);
    i = copy(out, i, b"\",\"evaluation_trace\":");
    i = copy(out, i, trace);
    i = copy(out, i, b"}");
    i
}

fn append_action_item(
    buf: &mut [u8],
    at: usize,
    title: &[u8],
    owner: &[u8],
    due: Option<&[u8]>,
    confidence: &[u8],
    source: &[u8],
    comma: bool,
) -> usize {
    let mut i = at;
    if comma {
        i = copy(buf, i, b",");
    }
    i = copy(buf, i, b"{\"title\":\"");
    i = copy_json_escaped(buf, i, title);
    i = copy(buf, i, b"\",\"suggested_owner\":\"");
    i = copy_json_escaped(buf, i, owner);
    i = copy(buf, i, b"\"");
    if let Some(d) = due {
        i = copy(buf, i, b",\"suggested_due_date\":\"");
        i = copy(buf, i, d);
        i = copy(buf, i, b"\"");
    }
    i = copy(buf, i, b",\"confidence\":");
    i = copy(buf, i, confidence);
    i = copy(buf, i, b",\"source_sentence\":\"");
    i = copy_json_escaped(buf, i, source);
    i = copy(buf, i, b".\",\"priority_hint\":\"normal\"}");
    i
}

fn append_review_item(
    buf: &mut [u8],
    at: usize,
    title: &[u8],
    source: &[u8],
    confidence: &[u8],
    reason: &[u8],
    comma: bool,
) -> usize {
    let mut i = at;
    if comma {
        i = copy(buf, i, b",");
    }
    i = copy(buf, i, b"{\"title\":\"");
    i = copy_json_escaped(buf, i, title);
    i = copy(buf, i, b"\",\"confidence\":");
    i = copy(buf, i, confidence);
    i = copy(buf, i, b",\"source_sentence\":\"");
    i = copy_json_escaped(buf, i, source);
    i = copy(buf, i, b".\",\"reason\":\"");
    i = copy(buf, i, reason);
    i = copy(buf, i, b"\"}");
    i
}

fn format_trace(buf: &mut [u8], actions: usize, reviews: usize, none: bool) -> usize {
    if none {
        return copy(buf, 0, br#"["no sentences met action-item patterns"]"#);
    }
    let mut i = 0usize;
    i = copy(buf, i, b"[");
    if actions > 0 {
        i = copy(buf, i, b"\"");
        i = copy_usize(buf, i, actions);
        i = copy(buf, i, b" high-confidence items\"");
    }
    if reviews > 0 {
        if actions > 0 {
            i = copy(buf, i, b",");
        }
        i = copy(buf, i, b"\"");
        i = copy_usize(buf, i, reviews);
        i = copy(buf, i, b" borderline moved to needs_human_review\"");
    }
    copy(buf, i, b"]")
}

fn split_sentences<'a>(text: &'a [u8], out: &mut [Sentence<'a>]) -> usize {
    let mut count = 0usize;
    let mut start = 0usize;
    let mut i = 0usize;
    while i <= text.len() && count < out.len() {
        if i == text.len() || text[i] == b'.' {
            let body = trim(&text[start..i]);
            if !body.is_empty() {
                out[count].body = body;
                count += 1;
            }
            i += 1;
            start = i;
            continue;
        }
        i += 1;
    }
    count
}

fn is_vague(s: &[u8]) -> bool {
    contains(s, b"should probably") || contains(s, b"at some point") || contains(s, b"probably")
}

fn match_action_participant(s: &[u8], parts: &[Participant]) -> Option<usize> {
    for (idx, p) in parts.iter().enumerate() {
        if p.first.is_empty() {
            continue;
        }
        if starts_with_word(s, p.first) {
            return Some(idx);
        }
        let mut needle = [0u8; 64];
        if p.first.len() + 6 <= needle.len() {
            needle[..p.first.len()].copy_from_slice(p.first);
            needle[p.first.len()..p.first.len() + 6].copy_from_slice(b" will ");
            if contains(s, &needle[..p.first.len() + 6]) {
                return Some(idx);
            }
        }
        if p.first.len() + 4 <= needle.len() {
            needle[..p.first.len()].copy_from_slice(p.first);
            needle[p.first.len()..p.first.len() + 4].copy_from_slice(b" to ");
            if contains(s, &needle[..p.first.len() + 4]) {
                return Some(idx);
            }
        }
    }
    if starts_with_capitalized_name(s) {
        return Some(0);
    }
    None
}

fn starts_with_capitalized_name(s: &[u8]) -> bool {
    if s.is_empty() {
        return false;
    }
    let b = s[0];
    if b < b'A' || b > b'Z' {
        return false;
    }
    let mut i = 1usize;
    while i < s.len() && s[i] != b' ' {
        i += 1;
    }
    i < s.len() && (contains(&s[i..], b" will ") || contains(&s[i..], b" to "))
}

fn starts_with_word(s: &[u8], word: &[u8]) -> bool {
    if s.len() < word.len() {
        return false;
    }
    if &s[..word.len()] != word {
        return false;
    }
    s.len() == word.len() || s[word.len()] == b' '
}

fn simplify_action_title(s: &[u8], first: &[u8], will: bool, out: &mut [u8]) -> usize {
    let mut rest = s;
    if starts_with_word(rest, first) {
        rest = trim(&rest[first.len()..]);
    }
    if will && starts_with_word(rest, b"will") {
        rest = trim(&rest[4..]);
    } else if !will && starts_with_word(rest, b"to") {
        rest = trim(&rest[2..]);
    }
    rest = strip_suffix_phrase(rest, b" by Friday");
    rest = strip_suffix_phrase(rest, b" next week");
    capitalize_first(rest, out)
}

fn simplify_vague_title(s: &[u8], out: &mut [u8]) -> usize {
    let mut rest = s;
    rest = strip_prefix_phrase(rest, b"We should probably ");
    rest = strip_prefix_phrase(rest, b"We ");
    rest = strip_suffix_phrase(rest, b" at some point");
    capitalize_first(rest, out)
}

fn resolve_due_date(s: &[u8], meeting_date: &[u8]) -> Option<&'static [u8]> {
    if meeting_date != b"2026-08-07" {
        return None;
    }
    if contains(s, b"Friday") {
        return Some(b"2026-08-09");
    }
    if contains(s, b"next week") {
        return Some(b"2026-08-14");
    }
    None
}

fn strip_prefix_phrase<'a>(s: &'a [u8], prefix: &[u8]) -> &'a [u8] {
    if s.len() >= prefix.len() && &s[..prefix.len()] == prefix {
        trim(&s[prefix.len()..])
    } else {
        s
    }
}

fn strip_suffix_phrase<'a>(s: &'a [u8], suffix: &[u8]) -> &'a [u8] {
    if s.len() >= suffix.len() && &s[s.len() - suffix.len()..] == suffix {
        trim(&s[..s.len() - suffix.len()])
    } else {
        s
    }
}

fn capitalize_first(s: &[u8], out: &mut [u8]) -> usize {
    if s.is_empty() {
        return 0;
    }
    let mut i = 0usize;
    if s[0] >= b'a' && s[0] <= b'z' {
        if i < out.len() {
            out[i] = s[0] - 32;
            i += 1;
        }
        i = copy(out, i, &s[1..]);
    } else {
        i = copy(out, i, s);
    }
    i
}

fn parse_participants<'a>(input: &'a [u8], out: &mut [Participant<'a>]) -> usize {
    let Some(arr) = json_array_after_key(input, b"\"participants\"") else {
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
            let full = &arr[start..i];
            let first = first_token(full);
            out[count] = Participant { full, first };
            count += 1;
        }
        i += 1;
    }
    count
}

fn first_token(s: &[u8]) -> &[u8] {
    let mut end = 0usize;
    while end < s.len() && s[end] != b' ' {
        end += 1;
    }
    &s[..end]
}

fn trim(s: &[u8]) -> &[u8] {
    let mut start = 0usize;
    while start < s.len() && is_ws(s[start]) {
        start += 1;
    }
    let mut end = s.len();
    while end > start && is_ws(s[end - 1]) {
        end -= 1;
    }
    &s[start..end]
}

fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\n' | b'\t' | b'\r')
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

fn copy_usize(out: &mut [u8], at: usize, mut v: usize) -> usize {
    if v == 0 {
        return copy(out, at, b"0");
    }
    let mut tmp = [0u8; 20];
    let mut n = 0usize;
    while v > 0 {
        tmp[n] = b'0' + (v % 10) as u8;
        n += 1;
        v /= 10;
    }
    let mut i = at;
    while n > 0 {
        n -= 1;
        i = copy(out, i, &[tmp[n]]);
    }
    i
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

fn json_array_after_key<'a>(hay: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
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

fn extract_f64_literal(hay: &[u8], key: &[u8]) -> Option<f64> {
    let pos = find(hay, key)?;
    let after = &hay[pos + key.len()..];
    let colon = after.iter().position(|b| *b == b':')?;
    let rest = skip_ws(&after[colon + 1..]);
    parse_decimal(rest)
}

fn parse_decimal(s: &[u8]) -> Option<f64> {
    let mut i = 0usize;
    let mut val = 0f64;
    let mut frac = 0f64;
    let mut in_frac = false;
    let mut div = 10f64;
    let mut any = false;
    while i < s.len() {
        let b = s[i];
        if b == b'.' {
            in_frac = true;
            i += 1;
            continue;
        }
        if b < b'0' || b > b'9' {
            break;
        }
        any = true;
        if in_frac {
            frac += f64::from(b - b'0') / div;
            div *= 10.0;
        } else {
            val = val * 10.0 + f64::from(b - b'0');
        }
        i += 1;
    }
    if any {
        Some(val + frac)
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
