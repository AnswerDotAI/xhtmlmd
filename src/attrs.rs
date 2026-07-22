use crate::ast::Attr;
use crate::entity::decode_entities;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttrLine {
    Ial(Attr),
    Ald(String, Attr),
}

pub fn parse_attr_line(line: &str, defs: &HashMap<String, Attr>) -> Option<AttrLine> {
    let t = line.trim();
    if !(t.starts_with("{:") && t.ends_with('}')) {
        return None;
    }
    let body = &t[2..t.len() - 1];
    if let Some(pos) = body.find(':') {
        let name = body[..pos].trim();
        if is_ref_name(name) {
            return Some(AttrLine::Ald(
                name.to_string(),
                parse_attrs_body(&body[pos + 1..], defs),
            ));
        }
    }
    Some(AttrLine::Ial(parse_attrs_body(body, defs)))
}

pub fn strip_trailing_attr(src: &str, defs: &HashMap<String, Attr>) -> (String, Attr) {
    let trimmed = src.trim_end();
    if !trimmed.ends_with('}') {
        return (src.trim().to_string(), Attr::default());
    }
    let Some(open) = last_attr_open(trimmed) else {
        return (src.trim().to_string(), Attr::default());
    };
    let body = &trimmed[open + 1..trimmed.len() - 1];
    let (body, had_colon) = match body.strip_prefix(':') {
        Some(rest) => (rest.trim(), true),
        None => (body.trim(), false),
    };
    if !looks_like_attrs(body, had_colon) {
        return (src.trim().to_string(), Attr::default());
    }
    (
        trimmed[..open].trim_end().to_string(),
        parse_attrs_body(body, defs),
    )
}

pub fn parse_braced_attr(src: &str, defs: &HashMap<String, Attr>) -> Option<(Attr, usize)> {
    if !src.starts_with('{') {
        return None;
    }
    let mut esc = false;
    let mut quote = None;
    for (i, ch) in src.char_indices().skip(1) {
        if esc {
            esc = false;
            continue;
        }
        if ch == '\\' {
            esc = true;
            continue;
        }
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            continue;
        }
        if ch == '\'' || ch == '"' {
            quote = Some(ch);
            continue;
        }
        if ch == '}' {
            let body = &src[1..i];
            let (body, had_colon) = match body.strip_prefix(':') {
                Some(rest) => (rest.trim(), true),
                None => (body.trim(), false),
            };
            if looks_like_attrs(body, had_colon) {
                return Some((parse_attrs_body(body, defs), i + 1));
            }
            return None;
        }
    }
    None
}

/// A leading Pandoc-style raw attribute `{=format}`: the name is one or more
/// ASCII alphanumerics, `-`, or `_`. Returns the name and bytes consumed.
pub fn raw_attr(src: &str) -> Option<(&str, usize)> {
    let rest = src.strip_prefix("{=")?;
    let end = rest.find('}')?;
    let name = &rest[..end];
    let ok = !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    ok.then_some((name, end + 3))
}

pub fn parse_span_ial(src: &str, defs: &HashMap<String, Attr>) -> Option<(Attr, usize)> {
    if !src.starts_with("{:") {
        return None;
    }
    let rest = &src[2..];
    let mut esc = false;
    let mut quote = None;
    for (off, ch) in rest.char_indices() {
        if esc {
            esc = false;
            continue;
        }
        if ch == '\\' {
            esc = true;
            continue;
        }
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            continue;
        }
        if ch == '\'' || ch == '"' {
            quote = Some(ch);
            continue;
        }
        if ch == '}' {
            let body = &rest[..off];
            return Some((parse_attrs_body(body, defs), off + 3));
        }
    }
    None
}

pub fn parse_attrs_body(body: &str, defs: &HashMap<String, Attr>) -> Attr {
    let mut out = Attr::default();
    for token in attr_tokens(body) {
        if let Some(referenced) = defs.get(&token) {
            out.merge(referenced);
            continue;
        }
        if token.starts_with('#') || token.starts_with('.') {
            parse_compact_attr_token(&token, &mut out);
        } else if let Some(pos) = token.find('=') {
            let key = token[..pos].trim();
            if key.is_empty() {
                continue;
            }
            let val = unquote(&token[pos + 1..]);
            out.set_pair(key.to_string(), val);
        }
    }
    out
}

fn parse_compact_attr_token(token: &str, out: &mut Attr) {
    let mut marker = None;
    let mut start = 0usize;
    let mut escaped = false;
    for (i, ch) in token.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '#' || ch == '.' {
            if let Some(marker) = marker {
                push_compact_attr(marker, &token[start..i], out);
            }
            marker = Some(ch);
            start = i + ch.len_utf8();
        }
    }
    if let Some(marker) = marker {
        push_compact_attr(marker, &token[start..], out);
    }
}

fn push_compact_attr(marker: char, raw: &str, out: &mut Attr) {
    if raw.is_empty() {
        return;
    }
    let value = unescape_attr(raw);
    if marker == '#' {
        out.id = Some(value);
    } else {
        out.push_class(value);
    }
}

pub fn parse_html_attrs(raw: &str) -> (Attr, Option<String>) {
    let mut attr = Attr::default();
    let mut markdown = None;
    for token in attr_tokens(raw) {
        if token.is_empty() {
            continue;
        }
        if let Some(pos) = token.find('=') {
            let key = token[..pos].trim().to_ascii_lowercase();
            let val = unquote(&token[pos + 1..]);
            if key == "markdown" {
                markdown = Some(val);
            } else {
                attr.set_pair(key, val);
            }
        } else if token.eq_ignore_ascii_case("markdown") {
            markdown = Some("1".to_string());
        } else {
            attr.set_pair(token.clone(), token);
        }
    }
    (attr, markdown)
}

pub fn parse_fence_info(
    info: &str,
    defs: &HashMap<String, Attr>,
) -> (String, Option<String>, Attr) {
    let mut attr = Attr::default();
    let info = info.trim();
    if info.is_empty() {
        return (String::new(), None, attr);
    }
    if let Some((a, n)) = parse_braced_attr(info, defs) {
        attr.merge(&a);
        let lang = attr.classes.first().cloned();
        return (info[..n].trim().to_string(), lang, attr);
    }
    let mut parts = info.splitn(2, char::is_whitespace);
    let token = parts.next().unwrap_or_default().trim().trim_matches('`');
    let rest = parts.next().unwrap_or_default().trim();
    if token.starts_with('.') || token.starts_with('#') {
        let mut dot_attr = Attr::default();
        let mut dot_rest = rest;
        if let Some(brace) = token.find('{') {
            let first = &token[..brace];
            if !first.is_empty()
                && let Some(a) = parse_synthetic_attrs(first, defs)
            {
                dot_attr.merge(&a);
            }
            if let Some((a, n)) = parse_braced_attr(&token[brace..], defs) {
                dot_attr.merge(&a);
                if !token[brace + n..].trim().is_empty() {
                    dot_rest = "invalid";
                }
            } else {
                dot_rest = "invalid";
            }
        } else if let Some(a) = parse_synthetic_attrs(token, defs) {
            dot_attr.merge(&a);
        } else {
            dot_rest = "invalid";
        }
        if let Some((a, n)) = parse_braced_attr(dot_rest, defs) {
            dot_attr.merge(&a);
            dot_rest = dot_rest[n..].trim();
        }
        if dot_rest.is_empty() {
            let lang = dot_attr.classes.first().cloned();
            return (info.to_string(), lang, dot_attr);
        }
    }
    let lang = decode_info_token(token);
    if let Some((a, _)) = parse_braced_attr(rest, defs) {
        if !lang.is_empty() {
            attr.push_class(lang.clone());
        }
        attr.merge(&a);
    }
    let first = if lang.is_empty() { None } else { Some(lang) };
    (info.to_string(), first, attr)
}

fn parse_synthetic_attrs(token: &str, defs: &HashMap<String, Attr>) -> Option<Attr> {
    parse_braced_attr(&format!("{{{token}}}"), defs)
        .and_then(|(attr, used)| (used == token.len() + 2).then_some(attr))
}

fn decode_info_token(s: &str) -> String {
    decode_entities(&unescape_punctuation(s))
}

fn unescape_punctuation(s: &str) -> String {
    let mut out = String::new();
    let mut esc = false;
    for ch in s.chars() {
        if esc {
            if ch.is_ascii_punctuation() {
                out.push(ch);
            } else {
                out.push('\\');
                out.push(ch);
            }
            esc = false;
        } else if ch == '\\' {
            esc = true;
        } else {
            out.push(ch);
        }
    }
    if esc {
        out.push('\\');
    }
    out
}

pub fn normalize_label(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .flat_map(char::to_lowercase)
        .collect()
}

pub fn scan_link_label(src: &str) -> Option<(String, usize)> {
    let rest = src.strip_prefix('[')?;
    let mut label = String::new();
    let mut escaped = false;
    let mut len = 0usize;
    for (off, ch) in rest.char_indices() {
        if escaped {
            label.push(ch);
            escaped = false;
            len += 1;
            if len > 999 {
                return None;
            }
            continue;
        }
        match ch {
            '\\' => {
                label.push(ch);
                escaped = true;
                len += 1;
            }
            '[' => return None,
            ']' => return Some((label, off + 2)),
            _ => {
                label.push(ch);
                len += 1;
            }
        }
        if len > 999 {
            return None;
        }
    }
    None
}

pub fn valid_link_label(label: &str, allow_empty: bool) -> bool {
    let mut escaped = false;
    let mut len = 0usize;
    let mut has_nonspace = false;
    for ch in label.chars() {
        if escaped {
            escaped = false;
            has_nonspace |= !ch.is_whitespace();
            len += 1;
            continue;
        }
        match ch {
            '\\' => {
                escaped = true;
                has_nonspace = true;
                len += 1;
            }
            '[' | ']' => return false,
            _ => {
                has_nonspace |= !ch.is_whitespace();
                len += 1;
            }
        }
        if len > 999 {
            return false;
        }
    }
    allow_empty || has_nonspace
}

fn last_attr_open(s: &str) -> Option<usize> {
    let mut esc = false;
    let mut quote = None;
    let mut last = None;
    for (i, ch) in s.char_indices() {
        if esc {
            esc = false;
            continue;
        }
        if ch == '\\' {
            esc = true;
            continue;
        }
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            continue;
        }
        if ch == '\'' || ch == '"' {
            quote = Some(ch);
            continue;
        }
        if ch == '{' {
            last = Some(i);
        }
    }
    last
}

fn looks_like_attrs(body: &str, had_colon: bool) -> bool {
    let b = body.trim();
    if b.is_empty() {
        return false;
    }
    if had_colon {
        return true;
    }
    b.starts_with('#')
        || b.starts_with('.')
        || b.split_whitespace()
            .next()
            .and_then(|t| t.find('='))
            .is_some_and(|pos| pos > 0)
}

fn attr_tokens(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut esc = false;
    let mut quote = None;
    for ch in body.chars() {
        if esc {
            cur.push(ch);
            esc = false;
            continue;
        }
        if ch == '\\' {
            esc = true;
            cur.push(ch);
            continue;
        }
        if let Some(q) = quote {
            cur.push(ch);
            if ch == q {
                quote = None;
            }
            continue;
        }
        if ch == '\'' || ch == '"' {
            quote = Some(ch);
            cur.push(ch);
        } else if ch.is_whitespace() {
            if !cur.is_empty() {
                out.push(cur.clone());
                cur.clear();
            }
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2 {
        let b = s.as_bytes();
        if (b[0] == b'"' && b[s.len() - 1] == b'"') || (b[0] == b'\'' && b[s.len() - 1] == b'\'') {
            return unescape_attr(&s[1..s.len() - 1]);
        }
    }
    unescape_attr(s)
}

fn unescape_attr(s: &str) -> String {
    let mut out = String::new();
    let mut esc = false;
    for ch in s.chars() {
        if esc {
            out.push(ch);
            esc = false;
        } else if ch == '\\' {
            esc = true;
        } else {
            out.push(ch);
        }
    }
    if esc {
        out.push('\\');
    }
    out
}

fn is_ref_name(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphanumeric() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}
