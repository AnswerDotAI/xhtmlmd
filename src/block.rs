use crate::ast::{
    Align, Attr, Block, DefinitionItem, Document, Footnote, Inline, LinkRef, ListItem,
};
use crate::attrs::{
    normalize_label, parse_attr_line, parse_fence_info, parse_html_attrs, strip_trailing_attr,
    AttrLine,
};
use crate::inline::{parse_inlines, InlineContext};
use crate::line::Line;
use crate::{MathMode, Options};
use html_escape::decode_html_entities;
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
        containers: true,
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
    containers: bool,
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
            if let Some((_, _, next)) = parse_link_ref_at(&self.lines, self.i) {
                self.i = next;
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
                            text.push_str(&strip_indent(self.line(), 4));
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
        if let Some(block) = self.indented_code() {
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
        if self.containers {
            if let Some(block) = self.container_block(depth) {
                return block;
            }
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
            containers: true,
        };
        let blocks = p.parse_blocks(depth);
        self.footnotes.extend(p.footnotes);
        self.warnings.extend(p.warnings);
        blocks
    }

    fn parse_leaf_lines(&mut self, lines: Vec<String>, depth: usize) -> Vec<Block> {
        let mut p = Parser {
            lines,
            i: 0,
            options: self.options.clone(),
            link_defs: self.link_defs.clone(),
            attr_defs: self.attr_defs.clone(),
            footnotes: Vec::new(),
            warnings: Vec::new(),
            containers: false,
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
        if !can_start_setext_text(self.line()) {
            return None;
        }
        let mut j = self.i + 1;
        while j < self.lines.len() {
            if let Some(level) = setext_underline(&self.lines[j]) {
                let body = self.lines[self.i..j]
                    .iter()
                    .map(|line| line.trim())
                    .collect::<Vec<_>>()
                    .join("\n");
                let (body, attrs) = if self.options.extensions.attributes {
                    strip_trailing_attr(&body, &self.attr_defs)
                } else {
                    (body, Attr::default())
                };
                let children = parse_inlines(&body, &self.ctx());
                self.i = j + 1;
                return Some(Block::Heading {
                    level,
                    attrs,
                    children,
                });
            }
            if !can_continue_setext_text(&self.lines[j]) {
                break;
            }
            j += 1;
        }
        None
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
        let (ch, len, fence_indent, info) =
            fence_start(self.line(), '`').or_else(|| fence_start(self.line(), '~'))?;
        let info = info.to_string();
        self.i += 1;
        let mut text = String::new();
        while self.i < self.lines.len() {
            if fence_close(self.line(), ch, len) {
                self.i += 1;
                break;
            }
            text.push_str(&strip_indent(self.line(), fence_indent));
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
        let end = html_block_end(t)?;
        let mut raw = String::new();
        match end {
            HtmlBlockEnd::BlankLine => {
                while self.i < self.lines.len() && !self.line().trim().is_empty() {
                    raw.push_str(self.line());
                    raw.push('\n');
                    self.i += 1;
                }
            }
            HtmlBlockEnd::Contains(pat) => {
                while self.i < self.lines.len() {
                    let lower = self.line().to_ascii_lowercase();
                    raw.push_str(self.line());
                    raw.push('\n');
                    self.i += 1;
                    if lower.contains(&pat) {
                        break;
                    }
                }
            }
        }
        Some(Block::Html { raw })
    }

    fn container_block(&mut self, depth: usize) -> Option<Block> {
        if !is_quote_line(self.line()) && list_marker(self.line()).is_none() {
            return None;
        }
        let attr_defs = self.attr_defs.clone();
        let options = self.options.clone();
        let mut builder = ContainerBuilder::new(&attr_defs, &options);
        while self.i < self.lines.len() {
            let line = self.line();
            if !builder.feed_line(line) {
                break;
            }
            self.i += 1;
        }
        let mut blocks = builder.finish(self, depth + 1);
        (!blocks.is_empty()).then(|| blocks.remove(0))
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
                text.push_str(&strip_indent(self.line(), 2));
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
                let mut next_i = self.i + 1;
                while next_i < self.lines.len() && self.lines[next_i].trim().is_empty() {
                    next_i += 1;
                }
                if next_i >= self.lines.len() || indent(&self.lines[next_i]) < 4 {
                    break;
                }
                text.push_str(&strip_indent(self.line(), 4));
                text.push('\n');
                self.i += 1;
                continue;
            }
            if indent(self.line()) < 4 {
                break;
            }
            text.push_str(&strip_indent(self.line(), 4));
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
            if lines.is_empty()
                && (parse_link_ref_at(&self.lines, self.i).is_some()
                    || footnote_start(self.line()).is_some())
            {
                break;
            }
            lines.push(self.line().trim_start().to_string());
            self.i += 1;
        }
        let joined = lines.join("\n").trim_end().to_string();
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

struct ContainerBuilder<'a> {
    nodes: Vec<BuildNode>,
    stack: Vec<usize>,
    attr_defs: &'a HashMap<String, Attr>,
    options: &'a Options,
    can_lazy: bool,
}

struct BuildNode {
    kind: BuildKind,
    children: Vec<usize>,
}

enum BuildKind {
    Root,
    BlockQuote {
        attrs: Attr,
    },
    List {
        attrs: Attr,
        ordered: bool,
        start: usize,
        kind: char,
    },
    ListItem {
        attrs: Attr,
        checked: Option<bool>,
        content_indent: usize,
        saw_blank: bool,
        loose: bool,
    },
    Lines(Vec<String>),
}

impl<'a> ContainerBuilder<'a> {
    fn new(attr_defs: &'a HashMap<String, Attr>, options: &'a Options) -> Self {
        Self {
            nodes: vec![BuildNode {
                kind: BuildKind::Root,
                children: Vec::new(),
            }],
            stack: vec![0],
            attr_defs,
            options,
            can_lazy: false,
        }
    }

    fn feed_line(&mut self, line: &str) -> bool {
        let mut content = line.to_string();
        self.match_containers(&mut content);
        if self.close_finished_list(&content) && self.at_root_after_complete_block() {
            return false;
        }
        if self.at_root_after_complete_block() {
            return false;
        }
        if content.trim().is_empty() {
            if self.stack.len() == 1 {
                return false;
            }
            self.mark_blank();
            self.append_line(String::new());
            self.can_lazy = false;
            return true;
        }
        if !self.open_starters(&mut content) {
            return false;
        }
        if self.stack.len() == 1 {
            return false;
        }
        self.mark_content();
        self.append_line(content.clone());
        self.can_lazy = is_lazy_paragraph_continuation(&content);
        true
    }

    fn match_containers(&mut self, content: &mut String) {
        let mut matched = 1;
        for depth in 1..self.stack.len() {
            let idx = self.stack[depth];
            match &self.nodes[idx].kind {
                BuildKind::BlockQuote { .. } => {
                    if is_quote_line(content) {
                        *content = strip_quote_marker(content);
                        matched = depth + 1;
                    } else if self.can_lazy && can_lazy_container_line(content) {
                        matched = depth + 1;
                    } else {
                        break;
                    }
                }
                BuildKind::List { .. } => matched = depth + 1,
                BuildKind::ListItem { content_indent, .. } => {
                    if content.trim().is_empty() {
                        if !self.item_has_content(idx) {
                            break;
                        }
                        matched = depth + 1;
                        content.clear();
                        break;
                    }
                    if indent(content) >= *content_indent {
                        *content = strip_indent(content, *content_indent);
                        matched = depth + 1;
                    } else if self.can_lazy && can_lazy_container_line(content) {
                        matched = depth + 1;
                    } else {
                        break;
                    }
                }
                BuildKind::Root | BuildKind::Lines(_) => break,
            }
        }
        self.stack.truncate(matched);
    }

    fn close_finished_list(&mut self, content: &str) -> bool {
        let Some(idx) = self.stack.last().copied() else {
            return false;
        };
        let BuildKind::List { .. } = self.nodes[idx].kind else {
            return false;
        };
        if thematic_line(content) {
            self.stack.pop();
            return true;
        }
        if let Some(marker) = list_marker(content) {
            if self.list_matches(idx, marker) {
                return false;
            }
        }
        self.stack.pop();
        true
    }

    fn open_starters(&mut self, content: &mut String) -> bool {
        loop {
            if self.container_depth() >= self.options.max_block_depth {
                break;
            }
            if thematic_line(content) {
                break;
            }
            if is_quote_line(content) {
                self.mark_content();
                let idx = self.open_node(BuildKind::BlockQuote {
                    attrs: Attr::default(),
                });
                self.stack.push(idx);
                *content = strip_quote_marker(content);
                continue;
            }
            let Some(marker) = list_marker(content) else {
                break;
            };
            let list_idx = if self
                .stack
                .last()
                .copied()
                .is_some_and(|idx| self.list_matches(idx, marker))
            {
                *self.stack.last().unwrap()
            } else {
                if self.at_root_after_complete_block() {
                    return false;
                }
                self.mark_content();
                let idx = self.open_node(BuildKind::List {
                    attrs: Attr::default(),
                    ordered: marker.ordered,
                    start: marker.start,
                    kind: marker.kind,
                });
                self.stack.push(idx);
                idx
            };
            let item = self.open_list_item(list_idx, marker);
            self.stack.push(item);
            *content = strip_marker_content(content, marker);
            self.prepare_item_head(item, content);
        }
        true
    }

    fn open_node(&mut self, kind: BuildKind) -> usize {
        let parent = *self.stack.last().unwrap();
        let idx = self.nodes.len();
        self.nodes.push(BuildNode {
            kind,
            children: Vec::new(),
        });
        self.nodes[parent].children.push(idx);
        idx
    }

    fn open_list_item(&mut self, list_idx: usize, marker: Marker) -> usize {
        self.mark_previous_item_loose(list_idx);
        let idx = self.nodes.len();
        self.nodes.push(BuildNode {
            kind: BuildKind::ListItem {
                attrs: Attr::default(),
                checked: None,
                content_indent: marker.content_indent,
                saw_blank: false,
                loose: false,
            },
            children: Vec::new(),
        });
        self.nodes[list_idx].children.push(idx);
        idx
    }

    fn mark_previous_item_loose(&mut self, list_idx: usize) {
        let Some(prev) = self.nodes[list_idx].children.last().copied() else {
            return;
        };
        if let BuildKind::ListItem {
            saw_blank, loose, ..
        } = &mut self.nodes[prev].kind
        {
            if *saw_blank {
                *loose = true;
            }
        }
    }

    fn prepare_item_head(&mut self, item: usize, content: &mut String) {
        let mut first = content.clone();
        let (attrs, checked) = prepare_list_item(
            std::slice::from_mut(&mut first),
            self.attr_defs,
            self.options,
        );
        if let BuildKind::ListItem {
            attrs: item_attrs,
            checked: item_checked,
            ..
        } = &mut self.nodes[item].kind
        {
            item_attrs.merge(&attrs);
            *item_checked = checked;
        }
        *content = first;
    }

    fn append_line(&mut self, line: String) {
        let parent = *self.stack.last().unwrap();
        if let Some(last) = self.nodes[parent].children.last().copied() {
            if let BuildKind::Lines(lines) = &mut self.nodes[last].kind {
                lines.push(line);
                return;
            }
        }
        let idx = self.nodes.len();
        self.nodes.push(BuildNode {
            kind: BuildKind::Lines(vec![line]),
            children: Vec::new(),
        });
        self.nodes[parent].children.push(idx);
    }

    fn mark_blank(&mut self) {
        if let Some(item) = self.current_list_item() {
            if let BuildKind::ListItem { saw_blank, .. } = &mut self.nodes[item].kind {
                *saw_blank = true;
            }
        }
    }

    fn mark_content(&mut self) {
        if let Some(item) = self.current_list_item() {
            if let BuildKind::ListItem {
                saw_blank, loose, ..
            } = &mut self.nodes[item].kind
            {
                if *saw_blank {
                    *loose = true;
                }
            }
        }
    }

    fn current_list_item(&self) -> Option<usize> {
        let idx = self.stack.last().copied()?;
        matches!(self.nodes[idx].kind, BuildKind::ListItem { .. }).then_some(idx)
    }

    fn item_has_content(&self, idx: usize) -> bool {
        self.nodes[idx]
            .children
            .iter()
            .any(|child| match &self.nodes[*child].kind {
                BuildKind::Lines(lines) => lines.iter().any(|line| !line.trim().is_empty()),
                _ => true,
            })
    }

    fn list_matches(&self, idx: usize, marker: Marker) -> bool {
        matches!(
            self.nodes[idx].kind,
            BuildKind::List {
                ordered,
                kind,
                ..
            } if ordered == marker.ordered && kind == marker.kind
        )
    }

    fn at_root_after_complete_block(&self) -> bool {
        self.stack.len() == 1 && !self.nodes[0].children.is_empty()
    }

    fn container_depth(&self) -> usize {
        self.stack.len().saturating_sub(1)
    }

    fn finish(&self, parser: &mut Parser, depth: usize) -> Vec<Block> {
        self.finish_children(0, parser, depth)
    }

    fn finish_children(&self, idx: usize, parser: &mut Parser, depth: usize) -> Vec<Block> {
        let mut out = Vec::new();
        for child in &self.nodes[idx].children {
            out.extend(self.finish_node(*child, parser, depth + 1));
        }
        out
    }

    fn finish_node(&self, idx: usize, parser: &mut Parser, depth: usize) -> Vec<Block> {
        match &self.nodes[idx].kind {
            BuildKind::Root => self.finish_children(idx, parser, depth),
            BuildKind::BlockQuote { attrs } => vec![Block::BlockQuote {
                attrs: attrs.clone(),
                children: self.finish_children(idx, parser, depth),
            }],
            BuildKind::List {
                attrs,
                ordered,
                start,
                ..
            } => {
                let mut tight = true;
                let mut items = Vec::new();
                for child in &self.nodes[idx].children {
                    if let Some((item, loose)) = self.finish_list_item(*child, parser, depth + 1) {
                        tight &= !loose;
                        items.push(item);
                    }
                }
                vec![Block::List {
                    attrs: attrs.clone(),
                    ordered: *ordered,
                    start: *start,
                    tight,
                    items,
                }]
            }
            BuildKind::ListItem { .. } => self.finish_children(idx, parser, depth),
            BuildKind::Lines(lines) => parser.parse_leaf_lines(lines.clone(), depth),
        }
    }

    fn finish_list_item(
        &self,
        idx: usize,
        parser: &mut Parser,
        depth: usize,
    ) -> Option<(ListItem, bool)> {
        let BuildKind::ListItem {
            attrs,
            checked,
            loose,
            ..
        } = &self.nodes[idx].kind
        else {
            return None;
        };
        Some((
            ListItem {
                attrs: attrs.clone(),
                checked: *checked,
                blocks: self.finish_children(idx, parser, depth),
            },
            *loose,
        ))
    }
}

fn can_lazy_container_line(line: &str) -> bool {
    !line.trim().is_empty()
        && !starts_block(line)
        && list_marker(line).is_none()
        && def_marker(line).is_none()
}

fn collect_link_defs(lines: &[String]) -> HashMap<String, LinkRef> {
    let scan_lines = lines
        .iter()
        .map(|line| strip_all_quote_markers(line))
        .collect::<Vec<_>>();
    let mut out = HashMap::new();
    let mut i = 0;
    let mut at_block_start = true;
    while i < scan_lines.len() {
        if scan_lines[i].trim().is_empty() {
            at_block_start = true;
            i += 1;
            continue;
        }
        if let Some((ch, len, _, _)) =
            fence_start(&scan_lines[i], '`').or_else(|| fence_start(&scan_lines[i], '~'))
        {
            i += 1;
            while i < scan_lines.len() {
                if fence_close(&scan_lines[i], ch, len) {
                    i += 1;
                    break;
                }
                i += 1;
            }
            at_block_start = true;
            continue;
        }
        if at_block_start {
            if let Some((label, lr, next)) = parse_link_ref_at(&scan_lines, i) {
                out.entry(normalize_label(&label)).or_insert(lr);
                i = next;
                at_block_start = true;
                continue;
            }
        }
        if !at_block_start {
            i += 1;
        } else if let Some((label, lr, next)) = parse_link_ref_at(&scan_lines, i) {
            out.entry(normalize_label(&label)).or_insert(lr);
            i = next;
            at_block_start = true;
        } else {
            i += 1;
            at_block_start = false;
        }
    }
    out
}

fn strip_all_quote_markers(line: &str) -> String {
    let mut rest = line;
    while let Some(next) = strip_one_quote_marker_slice(rest) {
        rest = next;
    }
    rest.to_string()
}

fn is_lazy_paragraph_continuation(line: &str) -> bool {
    let stripped = strip_all_quote_markers(line);
    !stripped.trim().is_empty()
        && indent(&stripped) <= 3
        && !starts_block(&stripped)
        && list_marker(&stripped).is_none()
        && def_marker(&stripped).is_none()
}

fn strip_one_quote_marker_slice(line: &str) -> Option<&str> {
    let first = Line::new(line).first_nonspace();
    if first.column > 3 || first.blank || !line[first.byte..].starts_with('>') {
        return None;
    }
    let rest = &line[first.byte + 1..];
    Some(
        rest.strip_prefix(' ')
            .or_else(|| rest.strip_prefix('\t'))
            .unwrap_or(rest),
    )
}

fn parse_link_ref_at(lines: &[String], i: usize) -> Option<(String, LinkRef, usize)> {
    let line = lines.get(i)?;
    if indent(line) > 3 {
        return None;
    }
    let t = line.trim_start();
    if !t.starts_with('[') || t.starts_with("[^") {
        return None;
    }
    let close = t.find("]:")?;
    let label = t[1..close].to_string();
    if label.trim().is_empty() {
        return None;
    }
    let mut rest = t[close + 2..].trim_start().to_string();
    let mut next = i + 1;
    while rest.is_empty() && next < lines.len() {
        if lines[next].trim().is_empty() {
            return None;
        }
        rest = lines[next].trim_start().to_string();
        next += 1;
    }
    if rest.is_empty() {
        return None;
    }
    let (url, used) = scan_link_ref_destination(&rest)?;
    let mut title = None;
    let raw_tail = &rest[used..];
    if !raw_tail.is_empty()
        && !raw_tail
            .chars()
            .next()
            .map(char::is_whitespace)
            .unwrap_or(false)
    {
        return None;
    }
    let tail = raw_tail.trim_start();
    if starts_definition_title(tail) {
        let (parsed, used_next) = scan_link_ref_title_lines(tail.to_string(), lines, next)?;
        title = Some(parsed);
        next = used_next;
    } else if !tail.trim().is_empty() {
        return None;
    } else if next < lines.len() && !lines[next].trim().is_empty() {
        let candidate = lines[next].trim_start();
        if starts_definition_title(candidate) {
            if let Some((parsed, used_next)) =
                scan_link_ref_title_lines(candidate.to_string(), lines, next + 1)
            {
                title = Some(parsed);
                next = used_next;
            }
        }
    }
    Some((label, LinkRef { url, title }, next))
}

fn scan_link_ref_destination(s: &str) -> Option<(String, usize)> {
    if let Some(rest) = s.strip_prefix('<') {
        let mut esc = false;
        for (idx, ch) in rest.char_indices() {
            if esc {
                esc = false;
                continue;
            }
            if ch == '\\' {
                esc = true;
                continue;
            }
            if ch == '>' {
                return Some((
                    decode_entities(&unescape_backslash_punctuation(&rest[..idx])),
                    idx + 2,
                ));
            }
            if ch == '\n' {
                return None;
            }
        }
        return None;
    }
    let mut end = 0;
    let mut depth = 0usize;
    let mut esc = false;
    for (idx, ch) in s.char_indices() {
        if esc {
            end = idx + ch.len_utf8();
            esc = false;
            continue;
        }
        if ch == '\\' {
            end = idx + ch.len_utf8();
            esc = true;
            continue;
        }
        if ch.is_whitespace() {
            break;
        }
        match ch {
            '(' => depth += 1,
            ')' if depth == 0 => break,
            ')' => depth -= 1,
            '<' => return None,
            _ => {}
        }
        end = idx + ch.len_utf8();
    }
    (end > 0 && depth == 0).then(|| {
        (
            decode_entities(&unescape_backslash_punctuation(&s[..end])),
            end,
        )
    })
}

fn starts_definition_title(s: &str) -> bool {
    matches!(
        s.trim_start().chars().next(),
        Some('"') | Some('\'') | Some('(')
    )
}

fn has_closing_definition_title(s: &str) -> bool {
    let s = s.trim_start();
    let Some(open) = s.chars().next() else {
        return false;
    };
    let close = match open {
        '"' => '"',
        '\'' => '\'',
        '(' => ')',
        _ => return false,
    };
    let mut esc = false;
    for ch in s[open.len_utf8()..].chars() {
        if esc {
            esc = false;
        } else if ch == '\\' {
            esc = true;
        } else if ch == close {
            return true;
        }
    }
    false
}

fn scan_link_ref_title_lines(
    mut title_src: String,
    lines: &[String],
    mut next: usize,
) -> Option<(String, usize)> {
    while !has_closing_definition_title(&title_src) && next < lines.len() {
        if lines[next].trim().is_empty() {
            return None;
        }
        title_src.push('\n');
        title_src.push_str(lines[next].trim_end());
        next += 1;
    }
    let (title, used) = scan_link_ref_title(&title_src)?;
    title_src[used..]
        .trim()
        .is_empty()
        .then_some((decode_entities(&title), next))
}

fn scan_link_ref_title(s: &str) -> Option<(String, usize)> {
    let s = s.trim_start();
    let open = s.chars().next()?;
    let close = match open {
        '"' => '"',
        '\'' => '\'',
        '(' => ')',
        _ => return None,
    };
    let mut out = String::new();
    let mut esc = false;
    let mut i = open.len_utf8();
    while i < s.len() {
        let ch = s[i..].chars().next().unwrap();
        if esc {
            if ch.is_ascii_punctuation() {
                out.push(ch);
            } else {
                out.push('\\');
                out.push(ch);
            }
            esc = false;
            i += ch.len_utf8();
            continue;
        }
        if ch == '\\' {
            esc = true;
            i += 1;
            continue;
        }
        if ch == close {
            return Some((out, i + ch.len_utf8()));
        }
        out.push(ch);
        i += ch.len_utf8();
    }
    None
}

fn unescape_backslash_punctuation(s: &str) -> String {
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

fn decode_entities(s: &str) -> String {
    decode_html_entities(s).into_owned()
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

fn fence_start(line: &str, ch: char) -> Option<(char, usize, usize, &str)> {
    let ind = indent(line);
    if ind > 3 {
        return None;
    }
    let t = line.trim_start();
    let b = if ch == '`' { b'`' } else { b'~' };
    let n = t.as_bytes().iter().take_while(|x| **x == b).count();
    let info = &t[n..];
    if n >= 3 {
        if ch == '`' && info.contains('`') {
            return None;
        }
        Some((ch, n, ind, info))
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

enum HtmlBlockEnd {
    BlankLine,
    Contains(String),
}

fn html_block_end(line: &str) -> Option<HtmlBlockEnd> {
    if !line.starts_with('<') {
        return None;
    }
    let lower = line.to_ascii_lowercase();
    if lower.starts_with("<!--") {
        return Some(HtmlBlockEnd::Contains("-->".to_string()));
    }
    if lower.starts_with("<?") {
        return Some(HtmlBlockEnd::Contains("?>".to_string()));
    }
    if lower.starts_with("<![cdata[") {
        return Some(HtmlBlockEnd::Contains("]]>".to_string()));
    }
    if line
        .as_bytes()
        .get(2)
        .map(|b| line.starts_with("<!") && b.is_ascii_alphabetic())
        .unwrap_or(false)
    {
        return Some(HtmlBlockEnd::Contains(">".to_string()));
    }
    let tag = html_tag_name(line)?;
    if matches!(tag.as_str(), "pre" | "script" | "style" | "textarea") {
        return Some(HtmlBlockEnd::Contains(format!("</{tag}>")));
    }
    if is_commonmark_block_html_tag(&tag) || is_complete_html_tag_line(line) {
        return Some(HtmlBlockEnd::BlankLine);
    }
    None
}

fn html_tag_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix("</").or_else(|| line.strip_prefix('<'))?;
    if rest.starts_with('!') || rest.starts_with('?') || rest.starts_with('/') {
        return None;
    }
    let mut end = 0;
    for (i, ch) in rest.char_indices() {
        if (i == 0 && ch.is_ascii_alphabetic())
            || (i > 0 && (ch.is_ascii_alphanumeric() || ch == '-'))
        {
            end = i + ch.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 {
        return None;
    }
    let next = rest[end..].chars().next().unwrap_or('>');
    if next.is_whitespace() || next == '>' || next == '/' {
        Some(rest[..end].to_ascii_lowercase())
    } else {
        None
    }
}

fn is_complete_html_tag_line(line: &str) -> bool {
    let t = line.trim();
    if t.starts_with("</") {
        let Some(tag) = html_tag_name(t) else {
            return false;
        };
        let Some(close) = t.find('>') else {
            return false;
        };
        return !tag.is_empty() && t[close + 1..].trim().is_empty();
    }
    parse_open_tag(t)
        .map(|open| t[open.end..].trim().is_empty())
        .unwrap_or(false)
}

fn is_commonmark_block_html_tag(tag: &str) -> bool {
    matches!(
        tag,
        "address"
            | "article"
            | "aside"
            | "base"
            | "basefont"
            | "blockquote"
            | "body"
            | "caption"
            | "center"
            | "col"
            | "colgroup"
            | "dd"
            | "details"
            | "dialog"
            | "dir"
            | "div"
            | "dl"
            | "dt"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "frame"
            | "frameset"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "head"
            | "header"
            | "hr"
            | "html"
            | "iframe"
            | "legend"
            | "li"
            | "link"
            | "main"
            | "menu"
            | "menuitem"
            | "nav"
            | "noframes"
            | "ol"
            | "optgroup"
            | "option"
            | "p"
            | "param"
            | "search"
            | "section"
            | "summary"
            | "table"
            | "tbody"
            | "td"
            | "tfoot"
            | "th"
            | "thead"
            | "title"
            | "tr"
            | "track"
            | "ul"
    )
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
fn strip_quote_marker(line: &str) -> String {
    let first = Line::new(line).first_nonspace();
    if first.column > 3 || first.blank || !line[first.byte..].starts_with('>') {
        return line.to_string();
    }
    let marker_end_byte = first.byte + 1;
    let marker_end_col = first.column + 1;
    let content_col = if line[marker_end_byte..]
        .chars()
        .next()
        .map(|c| c == ' ' || c == '\t')
        .unwrap_or(false)
    {
        marker_end_col + 1
    } else {
        marker_end_col
    };
    Line::new(line).strip_from(marker_end_byte, marker_end_col, content_col)
}

#[derive(Clone, Copy)]
struct Marker {
    ordered: bool,
    kind: char,
    start: usize,
    marker_end: usize,
    marker_end_col: usize,
    content_indent: usize,
}

fn list_marker(line: &str) -> Option<Marker> {
    let ind = indent(line);
    if ind > 3 {
        return None;
    }
    let byte_start = byte_at_column(line, ind)?;
    let t = &line[byte_start..];
    let bytes = t.as_bytes();
    if !bytes.is_empty()
        && matches!(bytes[0], b'-' | b'+' | b'*')
        && (bytes.len() == 1 || bytes[1].is_ascii_whitespace())
    {
        let marker_end = byte_start + 1;
        let marker_end_col = ind + 1;
        let content_indent = list_content_indent(line, marker_end, marker_end_col);
        return Some(Marker {
            ordered: false,
            kind: bytes[0] as char,
            start: 1,
            marker_end,
            marker_end_col,
            content_indent,
        });
    }
    let mut n = 0;
    while n < bytes.len() && bytes[n].is_ascii_digit() && n < 9 {
        n += 1;
    }
    if n > 0
        && n < bytes.len()
        && (bytes[n] == b'.' || bytes[n] == b')')
        && (n + 1 == bytes.len() || bytes[n + 1].is_ascii_whitespace())
    {
        let start = t[..n].parse::<usize>().unwrap_or(1);
        let marker_end = byte_start + n + 1;
        let marker_end_col = ind + n + 1;
        let content_indent = list_content_indent(line, marker_end, marker_end_col);
        return Some(Marker {
            ordered: true,
            kind: bytes[n] as char,
            start,
            marker_end,
            marker_end_col,
            content_indent,
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
    let mut first = lines[0].clone();
    let mut trimmed = first.trim_start();
    if options.extensions.attributes && trimmed.starts_with("{:") {
        if let Some(AttrLine::Ial(a)) = parse_attr_line(trimmed, defs) {
            attrs.merge(&a);
            first.clear();
            trimmed = "";
        } else if let Some(pos) = trimmed.find('}') {
            let attr_line = &trimmed[..=pos];
            if let Some(AttrLine::Ial(a)) = parse_attr_line(attr_line, defs) {
                attrs.merge(&a);
                first = trimmed[pos + 1..].trim_start().to_string();
                trimmed = &first;
            }
        }
    }
    if options.extensions.task_lists {
        let low = trimmed.to_ascii_lowercase();
        if low.starts_with("[ ] ") {
            checked = Some(false);
            first = trimmed[4..].to_string();
        } else if low.starts_with("[x] ") {
            checked = Some(true);
            first = trimmed[4..].to_string();
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
    if !cells.is_empty() {
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
        if dashes.is_empty() || !dashes.chars().all(|x| x == '-') {
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
    starts_block(line) || list_interrupts_paragraph(line) || def_marker(line).is_some()
}

fn list_interrupts_paragraph(line: &str) -> bool {
    let Some(marker) = list_marker(line) else {
        return false;
    };
    let content = strip_marker_content(line, marker);
    !content.trim().is_empty() && (!marker.ordered || marker.start == 1)
}
fn starts_block(line: &str) -> bool {
    if indent(line) > 3 {
        return false;
    }
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

fn can_start_setext_text(line: &str) -> bool {
    indent(line) <= 3
        && !line.trim().is_empty()
        && !starts_block(line)
        && list_marker(line).is_none()
        && def_marker(line).is_none()
}

fn can_continue_setext_text(line: &str) -> bool {
    indent(line) <= 3
        && !line.trim().is_empty()
        && !starts_block(line)
        && list_marker(line).is_none()
        && def_marker(line).is_none()
}

fn setext_underline(line: &str) -> Option<u8> {
    if indent(line) > 3 {
        return None;
    }
    let t = line.trim();
    if !t.is_empty() && t.chars().all(|c| c == '=') {
        Some(1)
    } else if !t.is_empty() && t.chars().all(|c| c == '-') {
        Some(2)
    } else {
        None
    }
}

fn indent(line: &str) -> usize {
    Line::new(line).indent()
}
fn byte_at_column(line: &str, target: usize) -> Option<usize> {
    Line::new(line).byte_at_column(target)
}

fn list_content_indent(line: &str, marker_end: usize, marker_end_col: usize) -> usize {
    let first = Line::new(line).first_nonspace_from(marker_end, marker_end_col);
    if first.blank {
        return marker_end_col + 1;
    }
    let col = first.column;
    let padding = col.saturating_sub(marker_end_col);
    if (1..=4).contains(&padding) {
        col
    } else {
        marker_end_col + 1
    }
}

fn strip_marker_content(line: &str, marker: Marker) -> String {
    strip_from_column(
        &line[marker.marker_end..],
        marker.marker_end_col,
        marker.content_indent,
    )
}

fn strip_indent(line: &str, n: usize) -> String {
    Line::new(line).strip_indent(n)
}

fn strip_from_column(line: &str, col: usize, n: usize) -> String {
    Line::new(line).strip_from(0, col, n)
}
