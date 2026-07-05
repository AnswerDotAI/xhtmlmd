//! Simplified tag balancing for rendered fragments. Markdown's raw-HTML rules
//! pass unbalanced user HTML through verbatim, so a rendered fragment is only
//! well-formed if the source's raw HTML was. This pass restores balance: stray
//! closing tags are dropped, a closing tag that skips over open elements first
//! closes them, and any elements still open at the end of the fragment are
//! closed there. Void elements are rewritten to self-closing form, and rawtext
//! elements (`script`, `style`, ...) are copied verbatim up to their matching
//! close. It deliberately does not implement HTML5 implied-end-tag rules (e.g.
//! `<p>` auto-close), attribute rewriting, or tag-case normalization.

use std::collections::HashMap;

fn is_void_tag(tag: &str) -> bool {
    matches!(
        tag.to_ascii_lowercase().as_str(),
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn is_rawtext_tag(tag: &str) -> bool {
    matches!(
        tag.to_ascii_lowercase().as_str(),
        "script" | "style" | "textarea" | "title" | "xmp" | "plaintext"
    )
}

fn tag_name_end(s: &str) -> Option<usize> {
    let first = s.chars().next()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    let mut end = first.len_utf8();
    while end < s.len() {
        let ch = s[end..].chars().next()?;
        if ch.is_ascii_alphanumeric() || ch == '-' {
            end += ch.len_utf8();
        } else {
            break;
        }
    }
    Some(end)
}

/// Offset of the `>` ending the tag whose contents start at `s`, skipping
/// quoted attribute values.
fn find_tag_close(s: &str) -> Option<usize> {
    let mut quote = None;
    for (i, ch) in s.char_indices() {
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            continue;
        }
        if ch == '\'' || ch == '"' {
            quote = Some(ch);
        } else if ch == '>' {
            return Some(i);
        }
    }
    None
}

/// Case-insensitive search for the closing tag of a rawtext element, returning
/// the offset just past its `>`.
fn find_rawtext_close(s: &str, tag: &str) -> Option<usize> {
    let mut i = 0;
    while i < s.len() {
        let Some(rel) = s[i..].find('<') else {
            return None;
        };
        i += rel;
        let rest = &s[i + 1..];
        let Some(rest) = rest.strip_prefix('/') else {
            i += 1;
            continue;
        };
        let Some(name_len) = tag_name_end(rest) else {
            i += 1;
            continue;
        };
        if rest[..name_len].eq_ignore_ascii_case(tag) {
            let next = rest[name_len..].chars().next();
            if matches!(next, Some('>') | None) || next.is_some_and(|ch| ch.is_whitespace()) {
                let end = rest[name_len..].find('>')?;
                return Some(i + 2 + name_len + end + 1);
            }
        }
        i += 1;
    }
    None
}

fn push_close(out: &mut String, tag: &str) {
    out.push_str("</");
    out.push_str(tag);
    out.push('>');
}

fn dec_count(counts: &mut HashMap<String, usize>, key: &str) {
    let Some(count) = counts.get_mut(key) else {
        return;
    };
    *count -= 1;
    if *count == 0 {
        counts.remove(key);
    }
}

/// Balance a rendered fragment (see module docs).
pub fn balance_fragment(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut stack: Vec<(String, String)> = Vec::new();
    let mut open_counts: HashMap<String, usize> = HashMap::new();
    let mut i = 0;
    while i < src.len() {
        let Some(rel) = src[i..].find('<') else {
            out.push_str(&src[i..]);
            break;
        };
        out.push_str(&src[i..i + rel]);
        i += rel;
        let rest = &src[i + 1..];
        if rest.starts_with("!--") {
            match rest.find("-->") {
                Some(end) => {
                    out.push_str(&src[i..i + 1 + end + 3]);
                    i += 1 + end + 3;
                }
                None => {
                    out.push_str(&src[i..]);
                    out.push_str("-->");
                    i = src.len();
                }
            }
            continue;
        }
        if rest.starts_with('!') || rest.starts_with('?') {
            let end = find_tag_close(rest).map(|e| e + 2).unwrap_or(rest.len() + 1);
            out.push_str(&src[i..i + end]);
            i += end;
            continue;
        }
        let closing = rest.starts_with('/');
        let name_start = usize::from(closing);
        let Some(name_len) = tag_name_end(&rest[name_start..]) else {
            out.push('<');
            i += 1;
            continue;
        };
        let name = &rest[name_start..name_start + name_len];
        let Some(close) = find_tag_close(rest) else {
            // Unterminated tag: escape the `<` so the remainder stays text.
            out.push_str("&lt;");
            i += 1;
            continue;
        };
        let tag_end = i + 1 + close + 1;
        if closing {
            let key = name.to_ascii_lowercase();
            if open_counts.get(&key).copied().unwrap_or(0) == 0 {
                i = tag_end; // stray close: drop it
                continue;
            }
            let Some(pos) = stack
                .iter()
                .rposition(|(_, open)| open == &key)
            else {
                i = tag_end; // stray close: drop it
                continue;
            };
            while stack.len() > pos + 1 {
                let (tag, key) = stack.pop().unwrap();
                dec_count(&mut open_counts, &key);
                push_close(&mut out, &tag);
            }
            let (_, key) = stack.pop().unwrap();
            dec_count(&mut open_counts, &key);
            out.push_str(&src[i..tag_end]);
            i = tag_end;
            continue;
        }
        let self_closing = rest[..close].trim_end().ends_with('/');
        if is_void_tag(name) {
            if self_closing {
                out.push_str(&src[i..tag_end]);
            } else {
                out.push_str(src[i..tag_end - 1].trim_end());
                out.push_str(" />");
            }
            i = tag_end;
            continue;
        }
        out.push_str(&src[i..tag_end]);
        i = tag_end;
        if self_closing {
            continue;
        }
        if is_rawtext_tag(name) {
            match find_rawtext_close(&src[i..], name) {
                Some(end) => {
                    out.push_str(&src[i..i + end]);
                    i += end;
                }
                None => {
                    out.push_str(&src[i..]);
                    push_close(&mut out, name);
                    i = src.len();
                }
            }
            continue;
        }
        let key = name.to_ascii_lowercase();
        *open_counts.entry(key.clone()).or_default() += 1;
        stack.push((name.to_string(), key));
    }
    let trailing_nl = out.ends_with('\n');
    if trailing_nl && !stack.is_empty() {
        out.pop();
    }
    while let Some((tag, _)) = stack.pop() {
        push_close(&mut out, &tag);
    }
    if trailing_nl && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}
