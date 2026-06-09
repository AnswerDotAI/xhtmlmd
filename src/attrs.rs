use crate::ast::Attr;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttrLine { Ial(Attr), Ald(String, Attr) }

pub fn parse_attr_line(line: &str, defs: &HashMap<String, Attr>) -> Option<AttrLine> {
    let t = line.trim();
    if !(t.starts_with("{:") && t.ends_with('}')) { return None; }
    let body = &t[2..t.len() - 1];
    if let Some(pos) = body.find(':') {
        let name = body[..pos].trim();
        if is_ref_name(name) {
            return Some(AttrLine::Ald(name.to_string(), parse_attrs_body(&body[pos + 1..], defs)));
        }
    }
    Some(AttrLine::Ial(parse_attrs_body(body, defs)))
}

pub fn strip_trailing_attr(src: &str, defs: &HashMap<String, Attr>) -> (String, Attr) {
    let trimmed = src.trim_end();
    if !trimmed.ends_with('}') { return (src.trim().to_string(), Attr::default()); }
    let Some(open) = last_attr_open(trimmed) else { return (src.trim().to_string(), Attr::default()); };
    let body = &trimmed[open + 1..trimmed.len() - 1];
    let body = body.strip_prefix(':').unwrap_or(body).trim();
    if !looks_like_attrs(body) { return (src.trim().to_string(), Attr::default()); }
    (trimmed[..open].trim_end().to_string(), parse_attrs_body(body, defs))
}

pub fn parse_braced_attr(src: &str, defs: &HashMap<String, Attr>) -> Option<(Attr, usize)> {
    if !src.starts_with('{') { return None; }
    let mut esc = false;
    let mut quote = None;
    for (i, ch) in src.char_indices().skip(1) {
        if esc { esc = false; continue; }
        if ch == '\\' { esc = true; continue; }
        if let Some(q) = quote {
            if ch == q { quote = None; }
            continue;
        }
        if ch == '\'' || ch == '"' { quote = Some(ch); continue; }
        if ch == '}' {
            let body = &src[1..i];
            let body = body.strip_prefix(':').unwrap_or(body).trim();
            if looks_like_attrs(body) { return Some((parse_attrs_body(body, defs), i + 1)); }
            return None;
        }
    }
    None
}

pub fn parse_span_ial(src: &str, defs: &HashMap<String, Attr>) -> Option<(Attr, usize)> {
    if !src.starts_with("{:") { return None; }
    let rest = &src[2..];
    let mut esc = false;
    let mut quote = None;
    for (off, ch) in rest.char_indices() {
        if esc { esc = false; continue; }
        if ch == '\\' { esc = true; continue; }
        if let Some(q) = quote {
            if ch == q { quote = None; }
            continue;
        }
        if ch == '\'' || ch == '"' { quote = Some(ch); continue; }
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
        if let Some(referenced) = defs.get(&token) { out.merge(referenced); continue; }
        if let Some(id) = token.strip_prefix('#') {
            if !id.is_empty() { out.id = Some(unescape_attr(id)); }
        } else if let Some(class) = token.strip_prefix('.') {
            if !class.is_empty() { out.push_class(unescape_attr(class)); }
        } else if let Some(pos) = token.find('=') {
            let key = token[..pos].trim();
            if key.is_empty() { continue; }
            let val = unquote(&token[pos + 1..]);
            out.set_pair(key.to_string(), val);
        }
    }
    out
}

pub fn parse_html_attrs(raw: &str) -> (Attr, Option<String>) {
    let mut attr = Attr::default();
    let mut markdown = None;
    for token in attr_tokens(raw) {
        if token.is_empty() { continue; }
        if let Some(pos) = token.find('=') {
            let key = token[..pos].trim().to_ascii_lowercase();
            let val = unquote(&token[pos + 1..]);
            if key == "markdown" { markdown = Some(val); }
            else { attr.set_pair(key, val); }
        } else if token.eq_ignore_ascii_case("markdown") {
            markdown = Some("1".to_string());
        } else {
            attr.set_pair(token.clone(), token);
        }
    }
    (attr, markdown)
}

pub fn parse_fence_info(info: &str, defs: &HashMap<String, Attr>) -> (String, Option<String>, Attr) {
    let mut attr = Attr::default();
    let info = info.trim();
    if info.is_empty() { return (String::new(), None, attr); }
    if let Some((a, n)) = parse_braced_attr(info, defs) {
        attr.merge(&a);
        let lang = attr.classes.first().cloned();
        return (info[..n].trim().to_string(), lang, attr);
    }
    let mut parts = info.splitn(2, char::is_whitespace);
    let lang = parts.next().unwrap_or_default().trim().trim_matches('`').to_string();
    if !lang.is_empty() { attr.push_class(lang.clone()); }
    if let Some(rest) = parts.next() {
        if let Some((a, _)) = parse_braced_attr(rest.trim(), defs) { attr.merge(&a); }
    }
    let first = if lang.is_empty() { None } else { Some(lang) };
    (info.to_string(), first, attr)
}

pub fn normalize_label(s: &str) -> String { s.split_whitespace().collect::<Vec<_>>().join(" ").to_ascii_lowercase() }

fn last_attr_open(s: &str) -> Option<usize> {
    let mut esc = false;
    let mut quote = None;
    let mut last = None;
    for (i, ch) in s.char_indices() {
        if esc { esc = false; continue; }
        if ch == '\\' { esc = true; continue; }
        if let Some(q) = quote {
            if ch == q { quote = None; }
            continue;
        }
        if ch == '\'' || ch == '"' { quote = Some(ch); continue; }
        if ch == '{' { last = Some(i); }
    }
    last
}

fn looks_like_attrs(body: &str) -> bool {
    let b = body.trim();
    b.is_empty() || b.starts_with('#') || b.starts_with('.') || b.contains('=') || b.split_whitespace().any(|t| t.starts_with('#') || t.starts_with('.') || t.contains('='))
}

fn attr_tokens(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut esc = false;
    let mut quote = None;
    for ch in body.chars() {
        if esc { cur.push(ch); esc = false; continue; }
        if ch == '\\' { esc = true; cur.push(ch); continue; }
        if let Some(q) = quote {
            cur.push(ch);
            if ch == q { quote = None; }
            continue;
        }
        if ch == '\'' || ch == '"' { quote = Some(ch); cur.push(ch); }
        else if ch.is_whitespace() {
            if !cur.is_empty() { out.push(cur.clone()); cur.clear(); }
        } else { cur.push(ch); }
    }
    if !cur.is_empty() { out.push(cur); }
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
        if esc { out.push(ch); esc = false; }
        else if ch == '\\' { esc = true; }
        else { out.push(ch); }
    }
    if esc { out.push('\\'); }
    out
}

fn is_ref_name(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else { return false; };
    (first.is_ascii_alphanumeric() || first == '_') && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}
