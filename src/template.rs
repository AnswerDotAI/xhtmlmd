use crate::{TemplateDelimiter, TemplateForm};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TemplateToken {
    pub syntax: String,
    pub source: String,
    pub body: String,
}

pub(crate) fn token_at(
    src: &str,
    start: usize,
    delimiters: &[TemplateDelimiter],
    block: bool,
) -> Option<(TemplateToken, usize)> {
    delimiters
        .iter()
        .filter(|d| !d.open.is_empty() && !d.close.is_empty())
        .filter(|d| {
            !matches!(
                (block, d.form),
                (true, TemplateForm::Inline) | (false, TemplateForm::Block)
            )
        })
        .filter(|d| src[start..].starts_with(&d.open))
        .max_by_key(|d| d.open.len())
        .and_then(|d| scan(src, start, d))
}

pub(crate) fn line_token(line: &str, delimiters: &[TemplateDelimiter]) -> Option<TemplateToken> {
    let trimmed = line.trim();
    let (token, end) = token_at(trimmed, 0, delimiters, true)?;
    (end == trimmed.len()).then_some(token)
}

fn scan(src: &str, start: usize, delimiter: &TemplateDelimiter) -> Option<(TemplateToken, usize)> {
    let body_start = start + delimiter.open.len();
    let body_end = match delimiter.balance {
        Some(balance) => balanced_end(src, body_start, &delimiter.close, balance)?,
        None => src[body_start..].find(&delimiter.close)? + body_start,
    };
    let end = body_end + delimiter.close.len();
    Some((
        TemplateToken {
            syntax: delimiter.syntax.clone(),
            source: src[start..end].to_string(),
            body: src[body_start..body_end].to_string(),
        },
        end,
    ))
}

fn balanced_end(
    src: &str,
    start: usize,
    close: &str,
    (open_balance, close_balance): (char, char),
) -> Option<usize> {
    let mut depth = 0;
    let mut quote = None;
    let mut escaped = false;
    for (offset, ch) in src[start..].char_indices() {
        let i = start + offset;
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if let Some(q) = quote {
            if ch == q {
                quote = None
            }
            continue;
        }
        if matches!(ch, '\'' | '"') {
            quote = Some(ch);
            continue;
        }
        if depth == 0 && src[i..].starts_with(close) {
            return Some(i);
        }
        if ch == open_balance {
            depth += 1;
        } else if ch == close_balance && depth > 0 {
            depth -= 1;
        }
    }
    None
}

/// Raw-text elements whose content the WHATWG parser never treats as markup;
/// template scanning skips their content along with tag internals and comments.
const RAW_TEXT_TAGS: [&str; 9] = [
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

/// Template tokens in the text between tags of raw HTML. Tag internals
/// (including attribute values), comments, CDATA sections, declarations, and
/// raw-text element content stay opaque.
pub(crate) fn html_tokens(
    src: &str,
    delimiters: &[TemplateDelimiter],
) -> Vec<(usize, usize, TemplateToken)> {
    let mut out = Vec::new();
    if delimiters.is_empty() {
        return out;
    }
    let mut i = 0;
    while i < src.len() {
        if src[i..].starts_with('<') {
            i = skip_markup(src, i);
            continue;
        }
        if let Some((token, end)) = token_at(src, i, delimiters, false) {
            out.push((i, end, token));
            i = end;
            continue;
        }
        i += src[i..].chars().next().map_or(1, char::len_utf8);
    }
    out
}

/// Advance past the markup construct starting with the `<` at `i`, or past the
/// bare `<` when nothing tag-like follows.
fn skip_markup(src: &str, i: usize) -> usize {
    let rest = &src[i + 1..];
    if rest.starts_with("!--") {
        return find_from(src, i + 4, "-->").map_or(src.len(), |j| j + 3);
    }
    if rest.starts_with("![CDATA[") {
        return find_from(src, i + 9, "]]>").map_or(src.len(), |j| j + 3);
    }
    if !rest.starts_with(|c: char| c.is_ascii_alphabetic() || matches!(c, '/' | '!' | '?')) {
        return i + 1;
    }
    let tag_end = find_from(src, i + 1, ">").map_or(src.len(), |j| j + 1);
    if let Some(name) = raw_text_tag(rest) {
        let mut j = tag_end;
        while let Some(k) = find_from(src, j, "</") {
            let after = &src[k + 2..];
            if starts_with_tag_name(after, name) {
                return k;
            }
            j = k + 2;
        }
        return src.len();
    }
    tag_end
}

fn raw_text_tag(rest: &str) -> Option<&'static str> {
    RAW_TEXT_TAGS
        .iter()
        .copied()
        .find(|name| starts_with_tag_name(rest, name))
}

/// ASCII-case-insensitive tag-name prefix test, byte-wise: `rest` may cut into
/// multi-byte text (e.g. a literal `</…>`), where a str slice would panic.
fn starts_with_tag_name(rest: &str, name: &str) -> bool {
    let b = rest.as_bytes();
    b.len() >= name.len()
        && b[..name.len()].eq_ignore_ascii_case(name.as_bytes())
        && !b.get(name.len()).is_some_and(|c| c.is_ascii_alphanumeric())
}

fn find_from(src: &str, from: usize, needle: &str) -> Option<usize> {
    if from > src.len() {
        return None;
    }
    src[from..].find(needle).map(|j| from + j)
}
