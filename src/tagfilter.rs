const FILTERED_TAGS: &[&str] = &[
    "title",
    "textarea",
    "style",
    "xmp",
    "iframe",
    "noembed",
    "noframes",
    "script",
    "plaintext",
];

pub fn tagfilter_html(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        let rest = &raw[i..];
        if is_tagfiltered_start(rest) {
            out.push_str("&lt;");
            i += 1;
        } else {
            let ch = rest.chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

pub fn is_tagfiltered_start(s: &str) -> bool {
    if !s.starts_with('<') {
        return false;
    }
    let mut rest = &s[1..];
    if let Some(stripped) = rest.strip_prefix('/') {
        rest = stripped;
    }
    let name_len = rest.bytes().take_while(|b| b.is_ascii_alphabetic()).count();
    if name_len == 0 {
        return false;
    }
    let name = &rest[..name_len];
    if !FILTERED_TAGS
        .iter()
        .any(|tag| name.eq_ignore_ascii_case(tag))
    {
        return false;
    }
    rest[name_len..]
        .chars()
        .next()
        .is_some_and(|ch| ch.is_whitespace() || matches!(ch, '>' | '/'))
}
