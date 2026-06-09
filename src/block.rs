use crate::ast::{
    Align, Attr, Block, DefinitionItem, Document, Footnote, Inline, LinkRef, ListItem,
};
use crate::attrs::{
    normalize_label, parse_attr_line, parse_fence_info, parse_html_attrs, strip_trailing_attr,
    AttrLine,
};
use crate::inline::{parse_inlines, InlineContext};
use crate::{MathMode, Options};
use std::collections::HashMap;

pub fn parse_document(src: &str, options: &Options) -> Document {
    let src = src.replace("\r\n", "\n").replace('\r', "\n");
    let lines = src.lines().map(|s| s.to_string()).collect::<Vec<_>>();
    let link_defs = collect_link_defs(&lines);
    let mut parser = Parser {
        lines,
        i: 0,
        options: options.clone(),
        link_defs,
        attr_defs: HashMap::new(),
        footnotes: Vec::new(),
        warnings: Vec::new(),
    };
    let blocks = parser.parse_blocks(0);
    Document {
        blocks,
        footnotes: parser.footnotes,
        warnings: parser.warnings,
    }
}

struct Parser {
    lines: Vec<String>,
    i: usize,
    options: Options,
    link_defs: HashMap<String, LinkRef>,
    attr_defs: HashMap<String, Attr>,
    footnotes: Vec<Footnote>,
    warnings: Vec<String>,
}

impl Parser {
    fn parse_blocks(&mut self, depth: usize) -> Vec<Block> {
        if depth > self.options.max_block_depth {
            return vec![Block::Paragraph {
                attrs: Attr::default(),
                children: vec![Inline::Text(self.lines[self.i..].join("\n"))],
            }];
        }
        let mut blocks = Vec::new();
        let mut pending = Attr::default();
        while self.i < self.lines.len() {
            if self.line().trim().is_empty() {
                self.i += 1;
                continue;
            }
            if self.line().trim() == "^" {
                self.i += 1;
                continue;
            }
            if self.options.extensions.attributes {
                if let Some(al) = parse_attr_line(self.line(), &self.attr_defs) {
                    match al {
                        AttrLine::Ald(name, attr) => {
                            self.attr_defs.entry(name).or_default().merge(&attr);
                        }
                        AttrLine::Ial(attr) => {
                            if let Some(last) = blocks.last_mut().and_then(Block::attrs_mut) {
                                last.merge(&attr);
                            } else {
                                pending.merge(&attr);
                            }
                        }
                    }
                    self.i += 1;
                    continue;
                }
            }
            if link_ref_line(self.line()).is_some() {
                self.i += 1;
                continue;
            }
            if self.options.extensions.footnotes {
                if let Some((label, first)) = footnote_start(self.line()) {
                    self.i += 1;
                    let mut text = first;
                    while self.i < self.lines.len() {
                        if self.line().trim().is_empty() {
                            text.push('\n');
                            self.i += 1;
                            continue;
                        }
                        if indent(self.line()) >= 4 {
                            text.push('\n');
                            text.push_str(strip_indent(self.line(), 4));
                            self.i += 1;
                        } else {
                            break;
                        }
                    }
                    let blocks = self.parse_nested(&text, depth + 1);
                    self.footnotes.push(Footnote { label, blocks });
                    continue;
                }
            }
            let mut block = self.parse_one(depth);
            if !pending.is_empty() {
                if let Some(dst) = block.attrs_mut() {
                    dst.merge(&pending);
                }
                pending = Attr::default();
            }
            blocks.push(block);
        }
        blocks
    }

    fn parse_one(&mut self, depth: usize) -> Block {
        if self.options.extensions.fenced_divs {
            if let Some(block) = self.fenced_div(depth) {
                return block;
            }
        }
        if self.options.extensions.fenced_code {
            if let Some(block) = self.fenced_code() {
                return block;
            }
        }
        if let Some(block) = self.block_math(depth) {
            return block;
        }
        if self.options.extensions.html_markdown {
            if let Some(block) = self.html_markdown_block(depth) {
                return block;
            }
        }
        if self.options.extensions.raw_html {
            if let Some(block) = self.raw_html_block() {
                return block;
            }
        }
        if let Some(block) = self.atx_heading() {
            return block;
        }
        if let Some(block) = self.setext_heading() {
            return block;
        }
        if let Some(block) = self.thematic_break() {
            return block;
        }
        if let Some(block) = self.blockquote(depth) {
            return block;
        }
        if let Some(block) = self.list(depth) {
            return block;
        }
        if self.options.extensions.tables {
            if let Some(block) = self.table() {
                return block;
            }
        }
        if self.options.extensions.definition_lists {
            if let Some(block) = self.definition_list(depth) {
                return block;
            }
        }
        if let Some(block) = self.indented_code() {
            return block;
        }
        self.paragraph()
    }

    fn ctx(&self) -> InlineContext<'_> {
        InlineContext {
            options: &self.options,
            attr_defs: &self.attr_defs,
            link_defs: &self.link_defs,
        }
    }
    fn line(&self) -> &str {
        &self.lines[self.i]
    }

    fn parse_nested(&mut self, text: &str, depth: usize) -> Vec<Block> {
        let lines = text.lines().map(|s| s.to_string()).collect::<Vec<_>>();
        let mut p = Parser {
            lines,
            i: 0,
            options: self.options.clone(),
            link_defs: self.link_defs.clone(),
            attr_defs: self.attr_defs.clone(),
            footnotes: Vec::new(),
            warnings: Vec::new(),
        };
        let blocks = p.parse_blocks(depth);
        self.footnotes.extend(p.footnotes);
        self.warnings.extend(p.warnings);
        blocks
    }

    fn atx_heading(&mut self) -> Option<Block> {
        let line = self.line();
        if indent(line) > 3 {
            return None;
        }
        let t = line.trim_start();
        let n = t.as_bytes().iter().take_while(|b| **b == b'#').count();
        if !(1..=6).contains(&n) {
            return None;
        }
        if t.len() > n && !t.as_bytes()[n].is_ascii_whitespace() {
            return None;
        }
        let mut body = t[n..].trim().to_string();
        if let Some(pos) = closing_hashes(&body) {
            body = body[..pos].trim_end().to_string();
        }
        let (body, attrs) = if self.options.extensions.attributes {
            strip_trailing_attr(&body, &self.attr_defs)
        } else {
            (body, Attr::default())
        };
        let children = parse_inlines(&body, &self.ctx());
        self.i += 1;
        Some(Block::Heading {
            level: n as u8,
            attrs,
            children,
        })
    }

    fn setext_heading(&mut self) -> Option<Block> {
        if self.i + 1 >= self.lines.len() || indent(self.line()) > 3 {
            return None;
        }
        let next = self.lines[self.i + 1].trim();
        let level = if next.chars().all(|c| c == '=') && next.len() >= 1 {
            1
        } else if next.chars().all(|c| c == '-') && next.len() >= 1 {
            2
        } else {
            return None;
        };
        let body = self.line().trim().to_string();
        if body.is_empty() {
            return None;
        }
        let (body, attrs) = if self.options.extensions.attributes {
            strip_trailing_attr(&body, &self.attr_defs)
        } else {
            (body, Attr::default())
        };
        let children = parse_inlines(&body, &self.ctx());
        self.i += 2;
        Some(Block::Heading {
            level,
            attrs,
            children,
        })
    }

    fn thematic_break(&mut self) -> Option<Block> {
        if indent(self.line()) > 3 {
            return None;
        }
        let s = self.line().trim().replace(' ', "").replace('\t', "");
        let mut chars = s.chars();
        let ch = chars.next()?;
        if (ch == '-' || ch == '*' || ch == '_') && s.len() >= 3 && chars.all(|c| c == ch) {
            self.i += 1;
            Some(Block::ThematicBreak {
                attrs: Attr::default(),
            })
        } else {
            None
        }
    }

    fn fenced_code(&mut self) -> Option<Block> {
        let (ch, len, info) =
            fence_start(self.line(), '`').or_else(|| fence_start(self.line(), '~'))?;
        let info = info.to_string();
        self.i += 1;
        let mut text = String::new();
        while self.i < self.lines.len() {
            if fence_close(self.line(), ch, len) {
                self.i += 1;
                break;
            }
            text.push_str(self.line());
            text.push('\n');
            self.i += 1;
        }
        let (info, lang, attrs) = parse_fence_info(&info, &self.attr_defs);
        Some(Block::CodeBlock {
            attrs,
            info,
            lang,
            text,
        })
    }

    fn fenced_div(&mut self, depth: usize) -> Option<Block> {
        let line = self.line();
        if indent(line) > 3 {
            return None;
        }
        let t = line.trim_start();
        let n = t.as_bytes().iter().take_while(|b| **b == b':').count();
        if n < 3 {
            return None;
        }
        let rest0 = t[n..].trim();
        if rest0.is_empty() || rest0.chars().all(|c| c == ':') {
            return None;
        }
        let rest = rest0.trim_end_matches(':').trim();
        let mut attrs = Attr::default();
        if rest.starts_with('{') {
            let (_, _, a) = parse_fence_info(rest, &self.attr_defs);
            attrs.merge(&a);
        } else {
            let class = rest.split_whitespace().next().unwrap_or(rest);
            attrs.push_class(class.trim_matches(':'));
            if let Some(brace) = rest.find('{') {
                let (_, _, a) = parse_fence_info(&rest[brace..], &self.attr_defs);
                attrs.merge(&a);
            }
        }
        self.i += 1;
        let mut inner = Vec::new();
        let mut div_depth = 1usize;
        while self.i < self.lines.len() {
            let t = self.line().trim();
            if t.starts_with(":::") && t.chars().all(|c| c == ':') {
                div_depth -= 1;
                self.i += 1;
                if div_depth == 0 {
                    break;
                }
                inner.push(String::new());
                continue;
            }
            if is_fenced_div_open(self.line()) {
                div_depth += 1;
            }
            inner.push(self.line().to_string());
            self.i += 1;
        }
        let children = self.parse_nested(&inner.join("\n"), depth + 1);
        Some(Block::Div { attrs, children })
    }

    fn block_math(&mut self, _depth: usize) -> Option<Block> {
        if self.options.math == MathMode::Off {
            return None;
        }
        let t = self.line().trim();
        if t == "\\[" {
            self.i += 1;
            let mut tex = String::new();
            while self.i < self.lines.len() {
                if self.line().trim() == "\\]" {
                    self.i += 1;
                    break;
                }
                tex.push_str(self.line());
                tex.push('\n');
                self.i += 1;
            }
            return Some(Block::Math {
                attrs: Attr::default(),
                display: true,
                tex: tex.trim_end().to_string(),
            });
        }
        if self.options.math == MathMode::Dollars && t == "$$" {
            self.i += 1;
            let mut tex = String::new();
            while self.i < self.lines.len() {
                if self.line().trim() == "$$" {
                    self.i += 1;
                    break;
                }
                tex.push_str(self.line());
                tex.push('\n');
                self.i += 1;
            }
            return Some(Block::Math {
                attrs: Attr::default(),
                display: true,
                tex: tex.trim_end().to_string(),
            });
        }
        None
    }

    fn html_markdown_block(&mut self, depth: usize) -> Option<Block> {
        let line = self.line();
        let open = parse_open_tag(line)?;
        let Some(markdown) = open.markdown else {
            return None;
        };
        if markdown != "1" && markdown != "block" && markdown != "span" {
            return None;
        }
        let close_tag = format!("</{}>", open.tag);
        let mut inner = String::new();
        let after_open = &line[open.end..];
        if let Some(pos) = after_open.to_ascii_lowercase().find(&close_tag) {
            inner.push_str(&after_open[..pos]);
            self.i += 1;
        } else {
            if !after_open.trim().is_empty() {
                inner.push_str(after_open);
                inner.push('\n');
            }
            self.i += 1;
            while self.i < self.lines.len() {
                let lower = self.line().to_ascii_lowercase();
                if let Some(pos) = lower.find(&close_tag) {
                    inner.push_str(&self.line()[..pos]);
                    self.i += 1;
                    break;
                }
                inner.push_str(self.line());
                inner.push('\n');
                self.i += 1;
            }
        }
        let children = if markdown == "span" {
            vec![Block::Paragraph {
                attrs: Attr::default(),
                children: parse_inlines(inner.trim(), &self.ctx()),
            }]
        } else {
            self.parse_nested(&inner, depth + 1)
        };
        Some(Block::HtmlContainer {
            tag: open.tag,
            attrs: open.attrs,
            children,
        })
    }

    fn raw_html_block(&mut self) -> Option<Block> {
        let t = self.line().trim_start();
        if !t.starts_with('<') {
            return None;
        }
        if t.starts_with("<!--") {
            let mut raw = String::new();
            while self.i < self.lines.len() {
                raw.push_str(self.line());
                raw.push('\n');
                let done = self.line().contains("-->");
                self.i += 1;
                if done {
                    break;
                }
            }
            return Some(Block::Html { raw });
        }
        if parse_open_tag(t).is_some()
            || t.starts_with("</")
            || t.starts_with("<!")
            || t.starts_with("<?")
        {
            let raw = self.line().to_string();
            self.i += 1;
            return Some(Block::Html {
                raw: format!("{raw}\n"),
            });
        }
        None
    }

    fn blockquote(&mut self, depth: usize) -> Option<Block> {
        if !is_quote_line(self.line()) {
            return None;
        }
        let mut inner = Vec::new();
        while self.i < self.lines.len() {
            if self.line().trim().is_empty() {
                inner.push(String::new());
                self.i += 1;
                continue;
            }
            if !is_quote_line(self.line()) {
                break;
            }
            inner.push(strip_quote_marker(self.line()).to_string());
            self.i += 1;
        }
        let children = self.parse_nested(&inner.join("\n"), depth + 1);
        Some(Block::BlockQuote {
            attrs: Attr::default(),
            children,
        })
    }

    fn list(&mut self, depth: usize) -> Option<Block> {
        let first = list_marker(self.line())?;
        let ordered = first.ordered;
        let start = first.start;
        let base_indent = first.indent;
        let mut items = Vec::new();
        let mut tight = true;
        while self.i < self.lines.len() {
            let Some(m) = list_marker(self.line()) else {
                break;
            };
            if m.ordered != ordered || m.indent != base_indent {
                break;
            }
            let mut item_lines = Vec::new();
            item_lines.push(self.line()[m.content_start..].to_string());
            self.i += 1;
            while self.i < self.lines.len() {
                if let Some(next) = list_marker(self.line()) {
                    if next.indent == base_indent && next.ordered == ordered {
                        break;
                    }
                }
                if !self.line().trim().is_empty()
                    && indent(self.line()) <= base_indent
                    && starts_block(self.line())
                {
                    break;
                }
                if self.line().trim().is_empty() {
                    let mut next_i = self.i + 1;
                    while next_i < self.lines.len() && self.lines[next_i].trim().is_empty() {
                        next_i += 1;
                    }
                    if next_i >= self.lines.len()
                        || (indent(&self.lines[next_i]) <= base_indent
                            && starts_block(&self.lines[next_i]))
                    {
                        break;
                    }
                    if let Some(next) = list_marker(&self.lines[next_i]) {
                        if next.indent == base_indent && next.ordered == ordered {
                            tight = false;
                            self.i = next_i;
                            break;
                        }
                    }
                    tight = false;
                    item_lines.push(String::new());
                    self.i += 1;
                    continue;
                }
                item_lines.push(strip_indent(self.line(), m.content_indent).to_string());
                self.i += 1;
            }
            let (attrs, checked) =
                prepare_list_item(&mut item_lines, &self.attr_defs, &self.options);
            let children = self.parse_nested(&item_lines.join("\n"), depth + 1);
            items.push(ListItem {
                attrs,
                checked,
                blocks: children,
            });
        }
        Some(Block::List {
            attrs: Attr::default(),
            ordered,
            start,
            tight,
            items,
        })
    }

    fn definition_list(&mut self, depth: usize) -> Option<Block> {
        if self.i + 1 >= self.lines.len() {
            return None;
        }
        let term_line = self.line().trim().to_string();
        if term_line.is_empty() || starts_block(&term_line) {
            return None;
        }
        let mut j = self.i + 1;
        if self.lines[j].trim().is_empty() {
            j += 1;
        }
        if j >= self.lines.len() || def_marker(&self.lines[j]).is_none() {
            return None;
        }
        self.i = j;
        let mut defs = Vec::new();
        while self.i < self.lines.len() {
            let Some(first) = def_marker(self.line()) else {
                break;
            };
            self.i += 1;
            let mut text = first;
            while self.i < self.lines.len() {
                if def_marker(self.line()).is_some() {
                    break;
                }
                if self.line().trim().is_empty() {
                    text.push('\n');
                    self.i += 1;
                    if self.i < self.lines.len() && def_marker(self.line()).is_some() {
                        break;
                    }
                    continue;
                }
                if indent(self.line()) <= 3
                    && (starts_block(self.line()) || list_marker(self.line()).is_some())
                {
                    break;
                }
                text.push('\n');
                text.push_str(strip_indent(self.line(), 2));
                self.i += 1;
            }
            defs.push(self.parse_nested(&text, depth + 1));
        }
        let term = parse_inlines(&term_line, &self.ctx());
        Some(Block::DefinitionList {
            attrs: Attr::default(),
            items: vec![DefinitionItem {
                term,
                definitions: defs,
            }],
        })
    }

    fn table(&mut self) -> Option<Block> {
        if self.i + 1 >= self.lines.len() {
            return None;
        }
        let header = split_table_row(self.line())?;
        let aligns = parse_table_separator(&self.lines[self.i + 1])?;
        if header.len() != aligns.len() {
            return None;
        }
        self.i += 2;
        let head = header
            .into_iter()
            .map(|c| parse_inlines(c.trim(), &self.ctx()))
            .collect::<Vec<_>>();
        let mut rows = Vec::new();
        while self.i < self.lines.len() {
            if self.line().trim().is_empty() {
                break;
            }
            if starts_block(self.line()) && !self.line().contains('|') {
                break;
            }
            let Some(mut row) = split_table_row(self.line()) else {
                break;
            };
            row.resize(aligns.len(), String::new());
            rows.push(
                row.into_iter()
                    .take(aligns.len())
                    .map(|c| parse_inlines(c.trim(), &self.ctx()))
                    .collect(),
            );
            self.i += 1;
        }
        Some(Block::Table {
            attrs: Attr::default(),
            aligns,
            head,
            rows,
        })
    }

    fn indented_code(&mut self) -> Option<Block> {
        if indent(self.line()) < 4 {
            return None;
        }
        let mut text = String::new();
        while self.i < self.lines.len() {
            if self.line().trim().is_empty() {
                text.push('\n');
                self.i += 1;
                continue;
            }
            if indent(self.line()) < 4 {
                break;
            }
            text.push_str(strip_indent(self.line(), 4));
            text.push('\n');
            self.i += 1;
        }
        Some(Block::CodeBlock {
            attrs: Attr::default(),
            info: String::new(),
            lang: None,
            text,
        })
    }

    fn paragraph(&mut self) -> Block {
        let mut lines = Vec::new();
        while self.i < self.lines.len() {
            if self.line().trim().is_empty() {
                break;
            }
            if !lines.is_empty() && paragraph_interrupts(self.line()) {
                break;
            }
            if self.options.extensions.attributes
                && parse_attr_line(self.line(), &self.attr_defs).is_some()
            {
                break;
            }
            if link_ref_line(self.line()).is_some() || footnote_start(self.line()).is_some() {
                break;
            }
            lines.push(self.line().trim().to_string());
            self.i += 1;
        }
        let joined = lines.join("\n");
        let (text, attrs) = if self.options.extensions.attributes {
            strip_trailing_attr(&joined, &self.attr_defs)
        } else {
            (joined, Attr::default())
        };
        Block::Paragraph {
            attrs,
            children: parse_inlines(&text, &self.ctx()),
        }
    }
}

fn collect_link_defs(lines: &[String]) -> HashMap<String, LinkRef> {
    let mut out = HashMap::new();
    for line in lines {
        if let Some((label, lr)) = link_ref_line(line) {
            out.entry(normalize_label(&label)).or_insert(lr);
        }
    }
    out
}

fn link_ref_line(line: &str) -> Option<(String, LinkRef)> {
    if indent(line) > 3 {
        return None;
    }
    let t = line.trim_start();
    if !t.starts_with('[') || t.starts_with("[^") {
        return None;
    }
    let close = t.find("]:")?;
    let label = t[1..close].to_string();
    let rest = t[close + 2..].trim();
    if label.is_empty() || rest.is_empty() {
        return None;
    }
    let (url, title) = parse_link_ref_dest(rest);
    Some((label, LinkRef { url, title }))
}

fn parse_link_ref_dest(rest: &str) -> (String, Option<String>) {
    let mut parts = rest.splitn(2, char::is_whitespace);
    let mut url = parts.next().unwrap_or_default().to_string();
    if url.starts_with('<') && url.ends_with('>') && url.len() > 1 {
        url = url[1..url.len() - 1].to_string();
    }
    let title = parts
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.trim_matches('"')
                .trim_matches('\'')
                .trim_matches('(')
                .trim_matches(')')
                .to_string()
        });
    (url, title)
}

fn footnote_start(line: &str) -> Option<(String, String)> {
    if indent(line) > 3 {
        return None;
    }
    let t = line.trim_start();
    if !t.starts_with("[^") {
        return None;
    }
    let pos = t.find("]:")?;
    let label = &t[2..pos];
    if label.is_empty() || label.contains(char::is_whitespace) {
        return None;
    }
    Some((label.to_string(), t[pos + 2..].trim_start().to_string()))
}

fn closing_hashes(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = bytes.len();
    while i > 0 && bytes[i - 1] == b'#' {
        i -= 1;
    }
    if i < bytes.len() && (i == 0 || bytes[i - 1].is_ascii_whitespace()) {
        Some(i)
    } else {
        None
    }
}

fn fence_start(line: &str, ch: char) -> Option<(char, usize, &str)> {
    if indent(line) > 3 {
        return None;
    }
    let t = line.trim_start();
    let b = if ch == '`' { b'`' } else { b'~' };
    let n = t.as_bytes().iter().take_while(|x| **x == b).count();
    if n >= 3 {
        Some((ch, n, &t[n..]))
    } else {
        None
    }
}

fn fence_close(line: &str, ch: char, len: usize) -> bool {
    if indent(line) > 3 {
        return false;
    }
    let t = line.trim_start();
    let b = if ch == '`' { b'`' } else { b'~' };
    let n = t.as_bytes().iter().take_while(|x| **x == b).count();
    n >= len && t[n..].trim().is_empty()
}

fn is_fenced_div_open(line: &str) -> bool {
    if indent(line) > 3 {
        return false;
    }
    let t = line.trim_start();
    let n = t.as_bytes().iter().take_while(|b| **b == b':').count();
    n >= 3 && !t[n..].trim().is_empty() && !t[n..].trim().chars().all(|c| c == ':')
}

#[derive(Clone)]
struct OpenTag {
    tag: String,
    attrs: Attr,
    markdown: Option<String>,
    end: usize,
}

fn parse_open_tag(line: &str) -> Option<OpenTag> {
    let start = line.find('<')?;
    let rest = &line[start + 1..];
    if rest.starts_with('/') || rest.starts_with('!') || rest.starts_with('?') {
        return None;
    }
    let mut name_end = 0;
    for (i, ch) in rest.char_indices() {
        if ch.is_ascii_alphanumeric() || ch == '-' {
            name_end = i + ch.len_utf8();
        } else {
            break;
        }
    }
    if name_end == 0 {
        return None;
    }
    let next = rest[name_end..].chars().next().unwrap_or('>');
    if !(next.is_whitespace() || next == '>' || next == '/') {
        return None;
    }
    let tag = rest[..name_end].to_ascii_lowercase();
    let close = rest.find('>')?;
    let raw_attrs = rest[name_end..close].trim().trim_end_matches('/').trim();
    let (attrs, markdown) = parse_html_attrs(raw_attrs);
    Some(OpenTag {
        tag,
        attrs,
        markdown,
        end: start + close + 2,
    })
}

fn is_quote_line(line: &str) -> bool {
    indent(line) <= 3 && line.trim_start().starts_with('>')
}
fn strip_quote_marker(line: &str) -> &str {
    let t = line.trim_start();
    let rest = &t[1..];
    rest.strip_prefix(' ').unwrap_or(rest)
}

#[derive(Clone, Copy)]
struct Marker {
    ordered: bool,
    start: usize,
    indent: usize,
    content_start: usize,
    content_indent: usize,
}

fn list_marker(line: &str) -> Option<Marker> {
    let ind = indent(line);
    if ind > 3 {
        return None;
    }
    let t = &line[ind.min(line.len())..];
    let bytes = t.as_bytes();
    if bytes.len() >= 2 && matches!(bytes[0], b'-' | b'+' | b'*') && bytes[1].is_ascii_whitespace()
    {
        let mut n = 1;
        while n < bytes.len() && bytes[n].is_ascii_whitespace() {
            n += 1;
        }
        return Some(Marker {
            ordered: false,
            start: 1,
            indent: ind,
            content_start: ind + n,
            content_indent: ind + n,
        });
    }
    let mut n = 0;
    while n < bytes.len() && bytes[n].is_ascii_digit() && n < 9 {
        n += 1;
    }
    if n > 0
        && n + 1 < bytes.len()
        && (bytes[n] == b'.' || bytes[n] == b')')
        && bytes[n + 1].is_ascii_whitespace()
    {
        let start = t[..n].parse::<usize>().unwrap_or(1);
        n += 1;
        while n < bytes.len() && bytes[n].is_ascii_whitespace() {
            n += 1;
        }
        return Some(Marker {
            ordered: true,
            start,
            indent: ind,
            content_start: ind + n,
            content_indent: ind + n,
        });
    }
    None
}

fn prepare_list_item(
    lines: &mut [String],
    defs: &HashMap<String, Attr>,
    options: &Options,
) -> (Attr, Option<bool>) {
    let mut attrs = Attr::default();
    let mut checked = None;
    if lines.is_empty() {
        return (attrs, checked);
    }
    let mut first = lines[0].trim_start().to_string();
    if options.extensions.attributes && first.starts_with("{:") {
        if let Some(AttrLine::Ial(a)) = parse_attr_line(&first, defs) {
            attrs.merge(&a);
            first.clear();
        } else if let Some(pos) = first.find('}') {
            let attr_line = &first[..=pos];
            if let Some(AttrLine::Ial(a)) = parse_attr_line(attr_line, defs) {
                attrs.merge(&a);
                first = first[pos + 1..].trim_start().to_string();
            }
        }
    }
    if options.extensions.task_lists {
        let low = first.to_ascii_lowercase();
        if low.starts_with("[ ] ") {
            checked = Some(false);
            first = first[4..].to_string();
        } else if low.starts_with("[x] ") {
            checked = Some(true);
            first = first[4..].to_string();
        }
    }
    lines[0] = first;
    (attrs, checked)
}

fn def_marker(line: &str) -> Option<String> {
    if indent(line) > 3 {
        return None;
    }
    let t = line.trim_start();
    let mut chars = t.chars();
    let ch = chars.next()?;
    if (ch == ':' || ch == '~') && chars.next().map(|c| c.is_whitespace()).unwrap_or(false) {
        Some(t[1..].trim_start().to_string())
    } else {
        None
    }
}

fn split_table_row(line: &str) -> Option<Vec<String>> {
    if !line.contains('|') {
        return None;
    }
    let mut cells = Vec::new();
    let mut cur = String::new();
    let mut esc = false;
    for ch in line.trim().chars() {
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
        if ch == '|' {
            cells.push(cur.trim().to_string());
            cur.clear();
        } else {
            cur.push(ch);
        }
    }
    cells.push(cur.trim().to_string());
    if cells.first().map(|s| s.is_empty()).unwrap_or(false) {
        cells.remove(0);
    }
    if cells.last().map(|s| s.is_empty()).unwrap_or(false) {
        cells.pop();
    }
    if cells.len() >= 2 {
        Some(cells)
    } else {
        None
    }
}

fn parse_table_separator(line: &str) -> Option<Vec<Align>> {
    let cells = split_table_row(line)?;
    let mut aligns = Vec::new();
    for cell in cells {
        let c = cell.trim();
        let left = c.starts_with(':');
        let right = c.ends_with(':');
        let dashes = c.trim_matches(':');
        if dashes.len() < 3 || !dashes.chars().all(|x| x == '-') {
            return None;
        }
        aligns.push(match (left, right) {
            (true, true) => Align::Center,
            (true, false) => Align::Left,
            (false, true) => Align::Right,
            _ => Align::None,
        });
    }
    Some(aligns)
}

fn paragraph_interrupts(line: &str) -> bool {
    starts_block(line) || list_marker(line).is_some() || def_marker(line).is_some()
}
fn starts_block(line: &str) -> bool {
    let t = line.trim_start();
    if t.is_empty() {
        return false;
    }
    t.starts_with('#')
        || t.starts_with('>')
        || t.starts_with("```")
        || t.starts_with("~~~")
        || t.starts_with(":::")
        || t.starts_with('<')
        || thematic_line(line)
}
fn thematic_line(line: &str) -> bool {
    if indent(line) > 3 {
        return false;
    }
    let s = line.trim().replace(' ', "").replace('\t', "");
    let mut chars = s.chars();
    let Some(ch) = chars.next() else {
        return false;
    };
    (ch == '-' || ch == '*' || ch == '_') && s.len() >= 3 && chars.all(|c| c == ch)
}
fn indent(line: &str) -> usize {
    line.chars().take_while(|c| *c == ' ').count()
}
fn strip_indent(line: &str, n: usize) -> &str {
    let cut = line
        .char_indices()
        .nth(n)
        .map(|(i, _)| i)
        .unwrap_or(line.len());
    &line[cut..]
}
