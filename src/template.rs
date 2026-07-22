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
        .filter(|d| match (block, d.form) {
            (true, TemplateForm::Inline) | (false, TemplateForm::Block) => false,
            _ => true,
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
