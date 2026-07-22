//! Smart punctuation (off by default): pandoc-style `---`/`--`/`...` and
//! quote curling, applied to Text inlines only, so code, math, and raw
//! content are never touched.

use crate::ast::{Block, Document, Inline, TableCellContent};

pub fn apply(doc: &mut Document) {
    walk_blocks(&mut doc.blocks, &mut |items| {
        for item in items.iter_mut() {
            if let Inline::Text(t) = item {
                *t = smarten(t);
            }
        }
    });
    for f in &mut doc.footnotes {
        walk_blocks(&mut f.blocks, &mut |items| {
            for item in items.iter_mut() {
                if let Inline::Text(t) = item {
                    *t = smarten(t);
                }
            }
        });
    }
}

fn opening_context(prev: Option<char>) -> bool {
    match prev {
        None => true,
        Some(c) => {
            c.is_whitespace() || matches!(c, '(' | '[' | '{' | '-' | '\u{2013}' | '\u{2014}')
        }
    }
}

fn smarten(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let prev = out.chars().next_back();
        match chars[i] {
            '-' if chars.get(i + 1) == Some(&'-') => {
                if chars.get(i + 2) == Some(&'-') {
                    out.push('\u{2014}');
                    i += 3;
                } else {
                    out.push('\u{2013}');
                    i += 2;
                }
            }
            '.' if chars.get(i + 1) == Some(&'.') && chars.get(i + 2) == Some(&'.') => {
                out.push('\u{2026}');
                i += 3;
            }
            '"' => {
                out.push(if opening_context(prev) {
                    '\u{201C}'
                } else {
                    '\u{201D}'
                });
                i += 1;
            }
            '\'' => {
                out.push(if opening_context(prev) {
                    '\u{2018}'
                } else {
                    '\u{2019}'
                });
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// Call `f` on every inline sequence in `blocks`, recursively.
fn walk_blocks(blocks: &mut [Block], f: &mut impl FnMut(&mut Vec<Inline>)) {
    for b in blocks {
        match b {
            Block::Paragraph { children, .. } | Block::Heading { children, .. } => {
                walk_inlines(children, f)
            }
            Block::BlockQuote { children, .. }
            | Block::HtmlContainer { children, .. }
            | Block::Div { children, .. } => walk_blocks(children, f),
            Block::List { items, .. } => {
                for item in items {
                    walk_blocks(&mut item.blocks, f);
                }
            }
            Block::DefinitionList { items, .. } => {
                for item in items {
                    for term in &mut item.terms {
                        walk_inlines(term, f);
                    }
                    for def in &mut item.definitions {
                        walk_blocks(&mut def.blocks, f);
                    }
                }
            }
            Block::Table {
                head,
                rows,
                foot,
                caption,
                ..
            } => {
                walk_inlines(caption, f);
                for row in head
                    .iter_mut()
                    .chain(rows.iter_mut())
                    .chain(foot.iter_mut())
                {
                    for cell in &mut row.cells {
                        match &mut cell.content {
                            TableCellContent::Inline(items) => walk_inlines(items, f),
                            TableCellContent::Blocks(blocks) => walk_blocks(blocks, f),
                        }
                    }
                }
            }
            Block::Figure { caption, image, .. } => {
                walk_inlines(caption, f);
                if let Inline::Image { alt, .. } = image {
                    walk_inlines(alt, f);
                }
            }
            _ => {}
        }
    }
}

/// Apply `f` to this sequence and to nested child sequences.
fn walk_inlines(items: &mut Vec<Inline>, f: &mut impl FnMut(&mut Vec<Inline>)) {
    f(items);
    for item in items {
        match item {
            Inline::Emph { children, .. }
            | Inline::Strong { children, .. }
            | Inline::Strike { children, .. }
            | Inline::Highlight { children, .. }
            | Inline::Link { children, .. }
            | Inline::Note { children }
            | Inline::Span { children, .. } => walk_inlines(children, f),
            Inline::Image { alt, .. } => walk_inlines(alt, f),
            _ => {}
        }
    }
}
