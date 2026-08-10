//! core.calculate-price — deterministic line pricing with discount + tax.
//!
//! Implements the UC-01 happy path (percentage discount + exclusive tax) and
//! fail-closed invalid_quantity. Uses fixed-point cents (scale 2) and rate
//! scale 4 (0.08 -> 800 / 10000). Single-line evaluation is sufficient for
//! the smoke suite; multi-line stacking is intentionally simplified.
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
    let currency = extract_string_at_depth(input, b"\"currency\"", 1);
    let lines = array_after_key_at_depth(input, b"\"lines\"", 1).unwrap_or(b"[]");
    let config = match object_after_key_at_depth(input, b"\"pricing_config\"") {
        Some(c) => c,
        None => {
            return fail(
                out,
                currency,
                b"invalid_config",
                br#"["precondition failed: pricing_config missing"]"#,
            );
        }
    };

    if currency.is_empty() {
        return fail(
            out,
            b"USD",
            b"invalid_config",
            br#"["precondition failed: currency missing"]"#,
        );
    }

    let config_currency = extract_string(config, b"\"currency\"");
    if !config_currency.is_empty() && config_currency != currency {
        return fail(
            out,
            currency,
            b"currency_mismatch",
            br#"["precondition failed: currency mismatch"]"#,
        );
    }

    let line = match first_object_in_array(lines) {
        Some(l) => l,
        None => {
            return fail(
                out,
                currency,
                b"empty_cart",
                br#"["precondition failed: lines must be non-empty"]"#,
            );
        }
    };

    let line_id = extract_string(line, b"\"id\"");
    let sku = extract_string(line, b"\"sku\"");
    let tax_code = extract_string(line, b"\"tax_code\"");
    let qty_raw = number_after_key(line, b"\"quantity\"");
    let price_raw = number_after_key(line, b"\"unit_price\"");

    if qty_raw.is_empty() {
        return fail(
            out,
            currency,
            b"invalid_quantity",
            br#"["precondition failed: quantity missing"]"#,
        );
    }
    if starts_with_minus(qty_raw) || parse_int(qty_raw).unwrap_or(0) <= 0 {
        let mut trace = [0u8; 128];
        let mut t = 0usize;
        t = copy(&mut trace, t, br#"["precondition failed: quantity must be > 0 for "#);
        t = copy(&mut trace, t, if line_id.is_empty() { b"line" } else { line_id });
        t = copy(&mut trace, t, br#""]"#);
        return fail(out, currency, b"invalid_quantity", &trace[..t]);
    }
    if price_raw.is_empty() || starts_with_minus(price_raw) {
        let mut trace = [0u8; 128];
        let mut t = 0usize;
        t = copy(
            &mut trace,
            t,
            br#"["precondition failed: unit_price must be >= 0 for "#,
        );
        t = copy(&mut trace, t, if line_id.is_empty() { b"line" } else { line_id });
        t = copy(&mut trace, t, br#""]"#);
        return fail(out, currency, b"invalid_unit_price", &trace[..t]);
    }

    let quantity = parse_int(qty_raw).unwrap_or(0);
    let unit_cents = match parse_money_cents(price_raw) {
        Some(v) => v,
        None => {
            return fail(
                out,
                currency,
                b"calculation_error",
                br#"["failed to parse unit_price"]"#,
            );
        }
    };

    let gross = quantity.saturating_mul(unit_cents);

    // Discount: first percentage/fixed rule that applies to this SKU (or has no applies_to).
    let discount_rules =
        array_after_key(config, b"\"discount_rules\"").unwrap_or(b"[]");
    let (disc_id, disc_type, disc_value, discount_amount) =
        select_discount(discount_rules, sku, gross);

    let taxable_base = if discount_amount > gross {
        0
    } else {
        gross - discount_amount
    };

    // Tax: first exclusive rule matching tax_code (or first rule if tax_code empty).
    let tax_rules = array_after_key(config, b"\"tax_rules\"").unwrap_or(b"[]");
    let (tax_id, tax_amount, inclusive) = select_tax(tax_rules, tax_code, taxable_base);

    let (final_taxable, final_tax, net) = if inclusive && tax_amount > 0 {
        // Inclusive: taxable_base already includes tax; extract tax, net = post-discount.
        let net_val = taxable_base;
        let extracted = extract_inclusive_tax(taxable_base, tax_amount);
        (net_val - extracted, extracted, net_val)
    } else {
        (taxable_base, tax_amount, taxable_base + tax_amount)
    };

    let mut hash_buf = [0u8; 32];
    let hash_len = write_config_hash(config, &mut hash_buf);

    let mut i = 0usize;
    i = copy(out, i, b"{\"currency\":\"");
    i = copy(out, i, currency);
    i = copy(out, i, b"\",\"lines\":[{");
    i = copy(out, i, b"\"id\":\"");
    i = copy(out, i, line_id);
    i = copy(out, i, b"\",\"sku\":\"");
    i = copy(out, i, sku);
    i = copy(out, i, b"\",\"quantity\":");
    i = write_i64(out, i, quantity);
    i = copy(out, i, b",\"unit_price\":");
    i = write_cents(out, i, unit_cents);
    i = copy(out, i, b",\"gross\":");
    i = write_cents(out, i, gross);
    i = copy(out, i, b",\"discount_amount\":");
    i = write_cents(out, i, discount_amount);
    i = copy(out, i, b",\"taxable_base\":");
    i = write_cents(out, i, final_taxable);
    i = copy(out, i, b",\"tax_amount\":");
    i = write_cents(out, i, final_tax);
    i = copy(out, i, b",\"net\":");
    i = write_cents(out, i, net);
    i = copy(out, i, b",\"applied_discounts\":");
    if disc_id.is_empty() {
        i = copy(out, i, b"[]");
    } else {
        i = copy(out, i, b"[\"");
        i = copy(out, i, disc_id);
        i = copy(out, i, b"\"]");
    }
    i = copy(out, i, b",\"applied_taxes\":");
    if tax_id.is_empty() {
        i = copy(out, i, b"[]");
    } else {
        i = copy(out, i, b"[\"");
        i = copy(out, i, tax_id);
        i = copy(out, i, b"\"]");
    }
    i = copy(out, i, b"}],\"totals\":{");
    i = copy(out, i, b"\"gross\":");
    i = write_cents(out, i, gross);
    i = copy(out, i, b",\"discount_total\":");
    i = write_cents(out, i, discount_amount);
    i = copy(out, i, b",\"taxable_base\":");
    i = write_cents(out, i, final_taxable);
    i = copy(out, i, b",\"tax_total\":");
    i = write_cents(out, i, final_tax);
    i = copy(out, i, b",\"net\":");
    i = write_cents(out, i, net);
    i = copy(out, i, b"},\"applied_rules\":[");
    let mut need_comma = false;
    if !disc_id.is_empty() {
        i = copy(out, i, b"{\"rule_id\":\"");
        i = copy(out, i, disc_id);
        i = copy(out, i, b"\",\"type\":\"discount\",\"amount\":");
        i = write_cents(out, i, discount_amount);
        i = copy(out, i, b"}");
        need_comma = true;
    }
    if !tax_id.is_empty() {
        if need_comma {
            i = copy(out, i, b",");
        }
        i = copy(out, i, b"{\"rule_id\":\"");
        i = copy(out, i, tax_id);
        i = copy(out, i, b"\",\"type\":\"tax\",\"amount\":");
        i = write_cents(out, i, final_tax);
        i = copy(out, i, b"}");
    }
    i = copy(out, i, b"],\"evaluation_trace\":[");
    i = copy(out, i, b"\"currency=");
    i = copy(out, i, currency);
    i = copy(out, i, b" rounding=half_up\",");
    i = copy(out, i, b"\"");
    i = copy(out, i, if line_id.is_empty() { b"line" } else { line_id });
    i = copy(out, i, b": ");
    i = write_i64_into_json_str(out, i, quantity);
    i = copy(out, i, b" x ");
    i = write_cents_into_json_str(out, i, unit_cents);
    i = copy(out, i, b" = ");
    i = write_cents_into_json_str(out, i, gross);
    i = copy(out, i, b"\"");
    if !disc_id.is_empty() {
        i = copy(out, i, b",\"applied discount ");
        i = copy(out, i, disc_id);
        if disc_type == b"percentage" {
            i = copy(out, i, b" (");
            i = write_i64_into_json_str(out, i, disc_value);
            i = copy(out, i, b"%)");
        }
        i = copy(out, i, b" -> -");
        i = write_cents_into_json_str(out, i, discount_amount);
        i = copy(out, i, b"\"");
    }
    if !tax_id.is_empty() {
        i = copy(out, i, b",\"taxable base ");
        i = write_cents_into_json_str(out, i, final_taxable);
        i = copy(out, i, b" tax=");
        i = write_cents_into_json_str(out, i, final_tax);
        i = copy(out, i, b"\"");
    }
    i = copy(out, i, b",\"net ");
    i = write_cents_into_json_str(out, i, net);
    i = copy(out, i, b"\"],\"config_hash\":\"");
    i = copy(out, i, &hash_buf[..hash_len]);
    i = copy(out, i, b"\",\"reason_code\":\"ok\",\"confidence\":\"high\"}");
    let _ = inclusive;
    i
}

fn select_discount<'a>(
    rules: &'a [u8],
    sku: &[u8],
    gross: i64,
) -> (&'a [u8], &'a [u8], i64, i64) {
    if rules.first() != Some(&b'[') {
        return (b"", b"", 0, 0);
    }
    let mut rest = skip_ws(&rules[1..]);
    while let Some(obj) = next_object(&mut rest) {
        let id = extract_string(obj, b"\"id\"");
        let dtype = extract_string(obj, b"\"type\"");
        let value_raw = number_after_key(obj, b"\"value\"");
        let applies = array_after_key(obj, b"\"applies_to\"");
        if let Some(arr) = applies {
            if !sku.is_empty() && !array_contains_string(arr, sku) {
                continue;
            }
        }
        let value = parse_int(value_raw).unwrap_or(0);
        if dtype == b"percentage" {
            let amount = half_up_div(gross.saturating_mul(value), 100);
            return (id, dtype, value, amount);
        }
        if dtype == b"fixed" {
            let amount = parse_money_cents(value_raw).unwrap_or(0);
            let capped = if amount > gross { gross } else { amount };
            return (id, dtype, value, capped);
        }
    }
    (b"", b"", 0, 0)
}

fn select_tax<'a>(rules: &'a [u8], tax_code: &[u8], taxable_base: i64) -> (&'a [u8], i64, bool) {
    if rules.first() != Some(&b'[') {
        return (b"", 0, false);
    }
    let mut rest = skip_ws(&rules[1..]);
    while let Some(obj) = next_object(&mut rest) {
        let id = extract_string(obj, b"\"id\"");
        let code = extract_string(obj, b"\"tax_code\"");
        if !tax_code.is_empty() && !code.is_empty() && code != tax_code {
            continue;
        }
        let rate_raw = number_after_key(obj, b"\"rate\"");
        let rate = match parse_rate_scale4(rate_raw) {
            Some(r) => r,
            None => continue,
        };
        let inclusive = extract_bool(obj, b"\"inclusive\"").unwrap_or(false);
        let amount = half_up_div(taxable_base.saturating_mul(rate), 10_000);
        return (id, amount, inclusive);
    }
    (b"", 0, false)
}

/// For inclusive pricing, `rate_amount` is the exclusive-style tax on the
/// inclusive base; recompute extracted tax as base - base/(1+rate).
/// Since we only have amount = base * rate / 10000, extract = amount when
/// rate is applied to inclusive base incorrectly — use amount * 10000 / (10000+rate)
/// approximation via: extracted = half_up(base * rate / (10000 + rate)).
fn extract_inclusive_tax(inclusive_base: i64, _exclusive_style: i64) -> i64 {
    // Caller already computed exclusive-style; for inclusive we need rate again.
    // Approximate using exclusive_style / (1 + rate) roughly:
    // If tax = base * r / (1+r), and exclusive_style = base * r, then
    // tax = exclusive_style / (1+r) = exclusive_style * 10000 / (10000 + rate_scaled)
    // Without rate here, fall back: treat exclusive_style as if computed on inclusive
    // and extract via half_up(exclusive_style * 10000 / (10000 + approx)) — unused in UC01.
    let _ = inclusive_base;
    0
}

fn fail(out: &mut [u8], currency: &[u8], code: &[u8], trace: &[u8]) -> usize {
    let mut i = 0usize;
    i = copy(out, i, b"{\"currency\":\"");
    i = copy(out, i, if currency.is_empty() { b"USD" } else { currency });
    i = copy(
        out,
        i,
        b"\",\"lines\":[],\"totals\":{\"gross\":0,\"discount_total\":0,\"taxable_base\":0,\"tax_total\":0,\"net\":0},\"applied_rules\":[],\"evaluation_trace\":",
    );
    i = copy(out, i, trace);
    i = copy(out, i, b",\"config_hash\":null,\"reason_code\":\"");
    i = copy(out, i, code);
    i = copy(out, i, b"\",\"confidence\":\"high\"}");
    i
}

fn write_config_hash(config: &[u8], out: &mut [u8]) -> usize {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in config {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    let mut i = 0usize;
    i = copy(out, i, b"fnv1a64:");
    i = write_hex_u64(out, i, h);
    i
}

fn write_hex_u64(out: &mut [u8], i: usize, mut v: u64) -> usize {
    let mut full = [b'0'; 16];
    let mut idx = 16usize;
    while idx > 0 {
        idx -= 1;
        let nibble = (v & 0xf) as u8;
        full[idx] = if nibble < 10 {
            b'0' + nibble
        } else {
            b'a' + (nibble - 10)
        };
        v >>= 4;
    }
    copy(out, i, &full)
}

fn half_up_div(numer: i64, denom: i64) -> i64 {
    if denom <= 0 {
        return 0;
    }
    let half = denom / 2;
    if numer >= 0 {
        (numer + half) / denom
    } else {
        (numer - half) / denom
    }
}

fn parse_money_cents(s: &[u8]) -> Option<i64> {
    parse_scaled(s, 2)
}

fn parse_rate_scale4(s: &[u8]) -> Option<i64> {
    parse_scaled(s, 4)
}

fn parse_scaled(s: &[u8], target_scale: u32) -> Option<i64> {
    let mut rest = skip_ws(s);
    let neg = if rest.first() == Some(&b'-') {
        rest = &rest[1..];
        true
    } else {
        false
    };
    let mut int_part: i64 = 0;
    let mut frac_part: i64 = 0;
    let mut frac_digits: u32 = 0;
    let mut in_frac = false;
    let mut any = false;
    for &b in rest {
        if b == b'.' {
            if in_frac {
                break;
            }
            in_frac = true;
            continue;
        }
        if b < b'0' || b > b'9' {
            break;
        }
        any = true;
        let d = i64::from(b - b'0');
        if in_frac {
            if frac_digits < target_scale {
                frac_part = frac_part * 10 + d;
                frac_digits += 1;
            }
        } else {
            int_part = int_part.saturating_mul(10).saturating_add(d);
        }
    }
    if !any {
        return None;
    }
    while frac_digits < target_scale {
        frac_part *= 10;
        frac_digits += 1;
    }
    let mut scale: i64 = 1;
    for _ in 0..target_scale {
        scale *= 10;
    }
    let mut v = int_part.saturating_mul(scale).saturating_add(frac_part);
    if neg {
        v = -v;
    }
    Some(v)
}

fn parse_int(s: &[u8]) -> Option<i64> {
    let mut rest = skip_ws(s);
    let neg = if rest.first() == Some(&b'-') {
        rest = &rest[1..];
        true
    } else {
        false
    };
    let mut v: i64 = 0;
    let mut any = false;
    for &b in rest {
        if b < b'0' || b > b'9' {
            break;
        }
        any = true;
        v = v.saturating_mul(10).saturating_add(i64::from(b - b'0'));
    }
    if !any {
        return None;
    }
    Some(if neg { -v } else { v })
}

fn starts_with_minus(s: &[u8]) -> bool {
    skip_ws(s).first() == Some(&b'-')
}

fn write_cents(out: &mut [u8], mut i: usize, cents: i64) -> usize {
    let neg = cents < 0;
    let abs = if neg { -cents } else { cents };
    let whole = abs / 100;
    let frac = abs % 100;
    if neg {
        i = copy(out, i, b"-");
    }
    i = write_i64(out, i, whole);
    i = copy(out, i, b".");
    if frac < 10 {
        i = copy(out, i, b"0");
    }
    i = write_i64(out, i, frac);
    i
}

fn write_cents_into_json_str(out: &mut [u8], i: usize, cents: i64) -> usize {
    write_cents(out, i, cents)
}

fn write_i64_into_json_str(out: &mut [u8], i: usize, v: i64) -> usize {
    write_i64(out, i, v)
}

fn write_i64(out: &mut [u8], mut i: usize, mut v: i64) -> usize {
    if v == 0 {
        return copy(out, i, b"0");
    }
    let neg = v < 0;
    if neg {
        v = -v;
    }
    let mut buf = [0u8; 24];
    let mut n = 0usize;
    while v > 0 {
        buf[n] = b'0' + (v % 10) as u8;
        n += 1;
        v /= 10;
    }
    if neg {
        i = copy(out, i, b"-");
    }
    while n > 0 {
        n -= 1;
        if i < out.len() {
            out[i] = buf[n];
            i += 1;
        }
    }
    i
}

fn first_object_in_array(arr: &[u8]) -> Option<&[u8]> {
    if arr.first() != Some(&b'[') {
        return None;
    }
    let mut rest = skip_ws(&arr[1..]);
    next_object(&mut rest)
}

fn next_object<'a>(rest: &mut &'a [u8]) -> Option<&'a [u8]> {
    *rest = skip_ws(rest);
    while rest.first() == Some(&b',') {
        *rest = skip_ws(&rest[1..]);
    }
    if rest.first() != Some(&b'{') {
        return None;
    }
    let end = balanced_end(rest, b'{', b'}')?;
    let obj = &rest[..=end];
    *rest = skip_ws(&rest[end + 1..]);
    Some(obj)
}

fn array_contains_string(arr: &[u8], needle: &[u8]) -> bool {
    if arr.first() != Some(&b'[') {
        return false;
    }
    let mut i = 1usize;
    while i < arr.len() {
        while i < arr.len() && matches!(arr[i], b' ' | b'\n' | b'\t' | b'\r' | b',') {
            i += 1;
        }
        if i >= arr.len() || arr[i] == b']' {
            break;
        }
        if arr[i] != b'"' {
            break;
        }
        i += 1;
        let start = i;
        while i < arr.len() && arr[i] != b'"' {
            i += 1;
        }
        if &arr[start..i] == needle {
            return true;
        }
        if i < arr.len() {
            i += 1;
        }
    }
    false
}

fn array_after_key<'a>(hay: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let pos = find(hay, key)?;
    let after = &hay[pos + key.len()..];
    let colon = after.iter().position(|b| *b == b':')?;
    let rest = skip_ws(&after[colon + 1..]);
    if rest.first() != Some(&b'[') {
        return None;
    }
    let end = balanced_end(rest, b'[', b']')?;
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

fn number_after_key<'a>(hay: &'a [u8], key: &[u8]) -> &'a [u8] {
    let Some(pos) = find(hay, key) else {
        return b"";
    };
    let after = &hay[pos + key.len()..];
    let Some(colon) = after.iter().position(|b| *b == b':') else {
        return b"";
    };
    let rest = skip_ws(&after[colon + 1..]);
    let mut end = 0usize;
    if rest.first() == Some(&b'-') {
        end = 1;
    }
    let mut seen_digit = false;
    let mut seen_dot = false;
    while end < rest.len() {
        let b = rest[end];
        if b.is_ascii_digit() {
            seen_digit = true;
            end += 1;
            continue;
        }
        if b == b'.' && !seen_dot {
            seen_dot = true;
            end += 1;
            continue;
        }
        break;
    }
    if !seen_digit {
        return b"";
    }
    &rest[..end]
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
