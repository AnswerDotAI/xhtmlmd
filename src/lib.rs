//! `xhtmlmd` is a small, dependency-free Markdown parser that targets
//! predictable, bounded-time parsing and a useful XHTML tree rather than exact
//! source round-tripping. The dialect is CommonMark/GFM for the core and GFM
//! features, with Pandoc choices for fenced divs, math, attributes, footnotes,
//! and definition lists when extension dialects disagree.

pub mod ast;
mod attrs;
mod block;
mod entity;
mod inline;
mod line;
mod python;
mod render;
mod tagfilter;

pub use ast::{Align, Attr, Block, DefinitionItem, Document, Footnote, Inline, LinkRef, ListItem};
pub use render::to_xhtml_document;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathMode {
    Off,
    On,
    Brackets,
    Dollars,
}

#[derive(Clone, Debug)]
pub struct Options {
    pub math: MathMode,
    pub tagfilter: bool,
    pub max_inline_depth: usize,
    pub max_block_depth: usize,
    pub max_link_paren_depth: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            math: MathMode::Brackets,
            tagfilter: false,
            max_inline_depth: 64,
            max_block_depth: 128,
            max_link_paren_depth: 32,
        }
    }
}

pub fn parse(src: &str, options: &Options) -> Document {
    block::parse_document(src, options)
}
pub fn to_xhtml(src: &str, options: &Options) -> String {
    render::to_xhtml_document(&parse(src, options))
}
