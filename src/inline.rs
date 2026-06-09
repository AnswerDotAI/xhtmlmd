use crate::ast::{Attr, Inline, LinkRef};
use crate::attrs::{normalize_label, parse_braced_attr, parse_span_ial};
use crate::{MathMode, Options};
use std::collections::HashMap;

pub struct InlineContext<'a> {
    pub options: &'a Options,
    pub attr_defs: &'a HashMap<String, Attr>,
    pub link_defs: &'a HashMap<String, LinkRef>,
}

pub fn parse_inlines(src: &str, ctx: &InlineContext<'_>) -> Vec<Inline> { coalesce(parse_inner(src, ctx, 0)) }

fn parse_inner(src: &str, ctx: &InlineContext<'_>, depth: usize) -> Vec<Inline> {
    if depth > ctx.options.max_inline_depth { return vec![Inline::Text(src.to_string())]; }
    let mut out = Vec::new();
    let mut text = String::new();
    let mut i = 0;
    let mut no_star = false;
    let mut no_under = false;
    let mut no_tilde = false;
    while i < src.len() {
        if starts(src, i, "\\[") && ctx.options.math != MathMode::Off {
            if let Some(end) = find_unescaped(src, i + 2, "\\]") {
                flush(&mut out, &mut text);
                let mut item = Inline::Math { attrs: Attr::default(), display: true, tex: src[i + 2..end].to_string() };
                i = push_with_attrs(&mut out, item, src, end + 2, ctx);
                continue;
            }
        }
        if starts(src, i, "\\(") && ctx.options.math != MathMode::Off {
            if let Some(end) = find_unescaped(src, i + 2, "\\)") {
                flush(&mut out, &mut text);
                let item = Inline::Math { attrs: Attr::default(), display: false, tex: src[i + 2..end].to_string() };
                i = push_with_attrs(&mut out, item, src, end + 2, ctx);
                continue;
            }
        }
        if ctx.options.math == MathMode::Dollars && starts(src, i, "$$") {
            if let Some(end) = find_unescaped(src, i + 2, "$$") {
                flush(&mut out, &mut text);
                let item = Inline::Math { attrs: Attr::default(), display: true, tex: src[i + 2..end].trim().to_string() };
                i = push_with_attrs(&mut out, item, src, end + 2, ctx);
                continue;
            }
        }
        if ctx.options.math == MathMode::Dollars && starts(src, i, "$") && can_open_dollar(src, i) {
            if let Some(end) = find_closing_dollar(src, i + 1) {
                flush(&mut out, &mut text);
                let item = Inline::Math { attrs: Attr::default(), display: false, tex: src[i + 1..end].to_string() };
                i = push_with_attrs(&mut out, item, src, end + 1, ctx);
                continue;
            }
        }
        if starts(src, i, "`") {
            if let Some((item, next)) = code_span(src, i) {
                flush(&mut out, &mut text);
                i = push_with_attrs(&mut out, item, src, next, ctx);
                continue;
            }
        }
        if ctx.options.extensions.footnotes && starts(src, i, "[^") {
            if let Some(end) = src[i + 2..].find(']') {
                let label = &src[i + 2..i + 2 + end];
                if valid_footnote_label(label) {
                    flush(&mut out, &mut text);
                    out.push(Inline::FootnoteRef { label: label.to_string() });
                    i += end + 3;
                    continue;
                }
            }
        }
        if starts(src, i, "![") || starts(src, i, "[") {
            if let Some((item, next)) = link_or_span(src, i, ctx, depth) {
                flush(&mut out, &mut text);
                i = push_with_attrs(&mut out, item, src, next, ctx);
                continue;
            }
        }
        if !no_star && starts(src, i, "**") {
            if let Some(end) = find_unescaped(src, i + 2, "**") {
                flush(&mut out, &mut text);
                let children = parse_inner(&src[i + 2..end], ctx, depth + 1);
                let item = Inline::Strong { attrs: Attr::default(), children };
                i = push_with_attrs(&mut out, item, src, end + 2, ctx);
                continue;
            } else { no_star = true; }
        }
        if !no_under && starts(src, i, "__") {
            if let Some(end) = find_unescaped(src, i + 2, "__") {
                flush(&mut out, &mut text);
                let children = parse_inner(&src[i + 2..end], ctx, depth + 1);
                let item = Inline::Strong { attrs: Attr::default(), children };
                i = push_with_attrs(&mut out, item, src, end + 2, ctx);
                continue;
            } else { no_under = true; }
        }
        if !no_tilde && ctx.options.extensions.strikethrough && starts(src, i, "~~") {
            if let Some(end) = find_unescaped(src, i + 2, "~~") {
                flush(&mut out, &mut text);
                let children = parse_inner(&src[i + 2..end], ctx, depth + 1);
                let item = Inline::Strike { attrs: Attr::default(), children };
                i = push_with_attrs(&mut out, item, src, end + 2, ctx);
                continue;
            } else { no_tilde = true; }
        }
        if !no_star && starts(src, i, "*") {
            if let Some(end) = find_unescaped(src, i + 1, "*") {
                flush(&mut out, &mut text);
                let children = parse_inner(&src[i + 1..end], ctx, depth + 1);
                let item = Inline::Emph { attrs: Attr::default(), children };
                i = push_with_attrs(&mut out, item, src, end + 1, ctx);
                continue;
            } else { no_star = true; }
        }
        if !no_under && starts(src, i, "_") && !is_intraword(src, i) {
            if let Some(end) = find_unescaped(src, i + 1, "_") {
                if !is_intraword(src, end) {
                    flush(&mut out, &mut text);
                    let children = parse_inner(&src[i + 1..end], ctx, depth + 1);
                    let item = Inline::Emph { attrs: Attr::default(), children };
                    i = push_with_attrs(&mut out, item, src, end + 1, ctx);
                    continue;
                }
            } else { no_under = true; }
        }
        if ctx.options.extensions.autolinks && starts(src, i, "<") {
            if let Some((item, next)) = angle_or_html(src, i) {
                flush(&mut out, &mut text);
                out.push(item);
                i = next;
                continue;
            }
        }
        if ctx.options.extensions.autolinks {
            if let Some((item, next)) = bare_autolink(src, i) {
                flush(&mut out, &mut text);
                out.push(item);
                i = next;
                continue;
            }
        }
        if starts(src, i, "\\") {
            if i + 1 < src.len() {
                let next = next_char(src, i + 1);
                if is_escapable(next) { text.push(next); i += 1 + next.len_utf8(); continue; }
            }
        }
        let ch = next_char(src, i);
        if ch == '\n' {
            if text.ends_with("  ") || text.ends_with('\\') {
                if text.ends_with('\\') { text.pop(); } else { while text.ends_with(' ') { text.pop(); } }
                flush(&mut out, &mut text);
                out.push(Inline::HardBreak);
            } else {
                flush(&mut out, &mut text);
                out.push(Inline::SoftBreak);
            }
            i += 1;
        } else {
            text.push(ch);
            i += ch.len_utf8();
        }
    }
    flush(&mut out, &mut text);
    out
}

fn push_with_attrs(out: &mut Vec<Inline>, mut item: Inline, src: &str, i: usize, ctx: &InlineContext<'_>) -> usize {
    let mut next = i;
    if ctx.options.extensions.attributes {
        while let Some((attr, n)) = parse_span_ial(&src[next..], ctx.attr_defs) {
            if let Some(dst) = item.attrs_mut() { dst.merge(&attr); next += n; }
            else { break; }
        }
    }
    out.push(item);
    next
}

fn link_or_span(src: &str, i: usize, ctx: &InlineContext<'_>, depth: usize) -> Option<(Inline, usize)> {
    let image = starts(src, i, "![");
    let open = if image { i + 1 } else { i };
    let close = find_matching_bracket(src, open + 1)?;
    let label_src = &src[open + 1..close];
    let after = close + 1;
    if after < src.len() && starts(src, after, "(") {
        let (inside, next) = paren_content(src, after + 1)?;
        let (url, title) = parse_link_destination_title(inside);
        let children = parse_inner(label_src, ctx, depth + 1);
        return Some((if image { Inline::Image { attrs: Attr::default(), alt: children, url, title } }
            else { Inline::Link { attrs: Attr::default(), children, url, title } }, next));
    }
    if !image && ctx.options.extensions.bracketed_spans {
        if let Some((attr, n)) = parse_braced_attr(&src[after..], ctx.attr_defs) {
            let children = parse_inner(label_src, ctx, depth + 1);
            return Some((Inline::Span { attrs: attr, children }, after + n));
        }
    }
    if !image && after < src.len() && starts(src, after, "[") {
        if let Some(end) = src[after + 1..].find(']') {
            let id = &src[after + 1..after + 1 + end];
            let key = if id.is_empty() { normalize_label(label_src) } else { normalize_label(id) };
            if let Some(lr) = ctx.link_defs.get(&key) {
                let children = parse_inner(label_src, ctx, depth + 1);
                return Some((Inline::Link { attrs: Attr::default(), children, url: lr.url.clone(), title: lr.title.clone() }, after + end + 2));
            }
        }
    }
    if !image {
        let key = normalize_label(label_src);
        if let Some(lr) = ctx.link_defs.get(&key) {
            let children = parse_inner(label_src, ctx, depth + 1);
            return Some((Inline::Link { attrs: Attr::default(), children, url: lr.url.clone(), title: lr.title.clone() }, after));
        }
    }
    Some((Inline::Text(src[i..after].to_string()), after))
}

fn code_span(src: &str, i: usize) -> Option<(Inline, usize)> {
    let run = count_run(src.as_bytes(), i, b'`');
    let mut j = i + run;
    while j < src.len() {
        if starts_run(src.as_bytes(), j, b'`', run) {
            let raw = &src[i + run..j];
            let mut txt = raw.replace('\n', " ").replace('\t', " ");
            if txt.starts_with(' ') && txt.ends_with(' ') && txt.chars().any(|c| c != ' ') { txt = txt[1..txt.len() - 1].to_string(); }
            return Some((Inline::Code { attrs: Attr::default(), text: txt }, j + run));
        }
        j += next_char(src, j).len_utf8();
    }
    None
}

fn angle_or_html(src: &str, i: usize) -> Option<(Inline, usize)> {
    let end = src[i + 1..].find('>')? + i + 1;
    if end - i > 1024 { return None; }
    let inside = &src[i + 1..end];
    if is_scheme_url(inside) { return Some((Inline::Autolink { url: inside.to_string(), text: inside.to_string(), email: false }, end + 1)); }
    if is_email(inside) { return Some((Inline::Autolink { url: format!("mailto:{inside}"), text: inside.to_string(), email: true }, end + 1)); }
    if looks_like_html(inside) { return Some((Inline::Html(src[i..end + 1].to_string()), end + 1)); }
    None
}

fn bare_autolink(src: &str, i: usize) -> Option<(Inline, usize)> {
    if starts(src, i, "http://") || starts(src, i, "https://") || starts(src, i, "www.") {
        if i > 0 && !is_boundary(prev_char(src, i)) { return None; }
        let mut end = i;
        while end < src.len() {
            let ch = next_char(src, end);
            if ch.is_whitespace() || ch == '<' { break; }
            end += ch.len_utf8();
        }
        while end > i {
            let ch = prev_char(src, end);
            if ".,;:!?)".contains(ch) { end -= ch.len_utf8(); } else { break; }
        }
        if end == i { return None; }
        let text = &src[i..end];
        let url = if text.starts_with("www.") { format!("http://{text}") } else { text.to_string() };
        return Some((Inline::Autolink { url, text: text.to_string(), email: false }, end));
    }
    if i == 0 || is_boundary(prev_char(src, i)) {
        let limit = src[i..].find(char::is_whitespace).map(|n| i + n).unwrap_or(src.len()).min(i + 255);
        let word = &src[i..limit];
        if word.contains('@') && is_email(word.trim_matches(|c: char| ".,;:!?)".contains(c))) {
            let trimmed = word.trim_matches(|c: char| ".,;:!?)".contains(c));
            let end = i + trimmed.len();
            return Some((Inline::Autolink { url: format!("mailto:{trimmed}"), text: trimmed.to_string(), email: true }, end));
        }
    }
    None
}

fn parse_link_destination_title(s: &str) -> (String, Option<String>) {
    let s = s.trim();
    if s.is_empty() { return (String::new(), None); }
    let mut tokens = split_link_tokens(s);
    if tokens.is_empty() { return (String::new(), None); }
    let mut url = tokens.remove(0);
    if url.starts_with('<') && url.ends_with('>') && url.len() > 1 { url = url[1..url.len() - 1].to_string(); }
    let title = if tokens.is_empty() { None } else { Some(tokens.join(" ").trim_matches('"').trim_matches('\'').trim_matches('(').trim_matches(')').to_string()) };
    (url, title.filter(|x| !x.is_empty()))
}

fn split_link_tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote = None;
    let mut esc = false;
    for ch in s.chars() {
        if esc { cur.push(ch); esc = false; continue; }
        if ch == '\\' { esc = true; cur.push(ch); continue; }
        if let Some(q) = quote {
            cur.push(ch);
            if ch == q { quote = None; }
        } else if ch == '"' || ch == '\'' || ch == '(' {
            quote = Some(if ch == '(' { ')' } else { ch }); cur.push(ch);
        } else if ch.is_whitespace() {
            if !cur.is_empty() { out.push(cur.clone()); cur.clear(); }
        } else { cur.push(ch); }
    }
    if !cur.is_empty() { out.push(cur); }
    out
}

fn paren_content(src: &str, mut i: usize) -> Option<(&str, usize)> {
    let start = i;
    let mut depth = 0usize;
    let mut esc = false;
    while i < src.len() {
        let ch = next_char(src, i);
        if esc { esc = false; i += ch.len_utf8(); continue; }
        if ch == '\\' { esc = true; i += 1; continue; }
        if ch == '(' { depth += 1; if depth > 32 { return None; } }
        else if ch == ')' {
            if depth == 0 { return Some((&src[start..i], i + 1)); }
            depth -= 1;
        }
        i += ch.len_utf8();
    }
    None
}

fn find_matching_bracket(src: &str, mut i: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut esc = false;
    while i < src.len() {
        let ch = next_char(src, i);
        if esc { esc = false; i += ch.len_utf8(); continue; }
        if ch == '\\' { esc = true; i += 1; continue; }
        if ch == '[' { depth += 1; }
        else if ch == ']' {
            if depth == 0 { return Some(i); }
            depth -= 1;
        }
        i += ch.len_utf8();
    }
    None
}

fn can_open_dollar(src: &str, i: usize) -> bool {
    if i + 1 >= src.len() { return false; }
    let r = next_char(src, i + 1);
    !r.is_whitespace() && r != '$'
}

fn find_closing_dollar(src: &str, mut i: usize) -> Option<usize> {
    while i < src.len() {
        let ch = next_char(src, i);
        if ch == '$' {
            let prev = prev_char(src, i);
            let next = if i + 1 < src.len() { Some(next_char(src, i + 1)) } else { None };
            if !prev.is_whitespace() && !next.map(|c| c.is_ascii_digit()).unwrap_or(false) { return Some(i); }
        }
        i += ch.len_utf8();
    }
    None
}

fn find_unescaped(src: &str, mut i: usize, pat: &str) -> Option<usize> {
    let mut esc = false;
    while i < src.len() {
        if !esc && src[i..].starts_with(pat) { return Some(i); }
        let ch = next_char(src, i);
        esc = !esc && ch == '\\';
        if ch != '\\' { esc = false; }
        i += ch.len_utf8();
    }
    None
}

fn starts(src: &str, i: usize, pat: &str) -> bool { i <= src.len() && src[i..].starts_with(pat) }
fn next_char(src: &str, i: usize) -> char { src[i..].chars().next().unwrap() }
fn prev_char(src: &str, i: usize) -> char { src[..i].chars().next_back().unwrap_or('\0') }
fn count_run(bytes: &[u8], mut i: usize, b: u8) -> usize { let start = i; while i < bytes.len() && bytes[i] == b { i += 1; } i - start }
fn starts_run(bytes: &[u8], i: usize, b: u8, n: usize) -> bool { i + n <= bytes.len() && bytes[i..i + n].iter().all(|x| *x == b) }
fn valid_footnote_label(s: &str) -> bool { !s.is_empty() && !s.chars().any(|c| c.is_whitespace() || c == '^' || c == '[' || c == ']') }
fn is_scheme_url(s: &str) -> bool { s.starts_with("http://") || s.starts_with("https://") || s.starts_with("ftp://") }
fn is_email(s: &str) -> bool { let Some(at) = s.find('@') else { return false; }; at > 0 && s[at + 1..].contains('.') && !s.contains(char::is_whitespace) }
fn looks_like_html(s: &str) -> bool { s.starts_with('!') || s.starts_with('?') || s.starts_with('/') || s.chars().next().map(|c| c.is_ascii_alphabetic()).unwrap_or(false) }
fn is_boundary(ch: char) -> bool { ch == '\0' || ch.is_whitespace() || "([{:;'\"".contains(ch) }
fn is_escapable(ch: char) -> bool { "\\`*_{ }[]()#+-.!<>$|~:".contains(ch) }
fn is_intraword(src: &str, i: usize) -> bool { prev_char(src, i).is_alphanumeric() && i + 1 < src.len() && next_char(src, i + 1).is_alphanumeric() }
fn flush(out: &mut Vec<Inline>, text: &mut String) { if !text.is_empty() { out.push(Inline::Text(std::mem::take(text))); } }

fn coalesce(items: Vec<Inline>) -> Vec<Inline> {
    let mut out: Vec<Inline> = Vec::with_capacity(items.len());
    for item in items {
        match (out.last_mut(), item) {
            (Some(Inline::Text(a)), Inline::Text(b)) => a.push_str(&b),
            (_, x) => out.push(x),
        }
    }
    out
}
