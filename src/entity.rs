use html_escape::decode_html_entities;

pub fn decode_entities(s: &str) -> String {
    let s = preprocess_entities(s);
    decode_html_entities(&s).into_owned()
}

fn preprocess_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let rest = &s[i..];
        if rest.starts_with("&ngE;") {
            out.push_str("≧\u{0338}");
            i += 5;
            continue;
        }
        if let Some((ch, next)) = numeric_ref(rest) {
            out.push(ch);
            i += next;
            continue;
        }
        let ch = rest.chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn numeric_ref(s: &str) -> Option<(char, usize)> {
    let rest = s.strip_prefix("&#")?;
    let (digits, radix, offset) = if let Some(rest) = rest.strip_prefix(['x', 'X']) {
        (rest, 16, 3)
    } else {
        (rest, 10, 2)
    };
    let end = digits.find(';')?;
    if end == 0 {
        return None;
    }
    let raw = &digits[..end];
    if !raw.chars().all(|ch| ch.is_digit(radix)) {
        return None;
    }
    let n = u32::from_str_radix(raw, radix).ok()?;
    let ch = if n == 0 {
        '\u{FFFD}'
    } else {
        char::from_u32(n)?
    };
    Some((ch, offset + end + 1))
}
