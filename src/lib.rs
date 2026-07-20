//! `xhtmlmd` is a small, dependency-free Markdown parser that targets
//! predictable, bounded-time parsing and a useful XHTML tree rather than exact
//! source round-tripping. The dialect is CommonMark/GFM for the core and GFM
//! features, with Pandoc choices for fenced divs, math, attributes, footnotes,
//! and definition lists when extension dialects disagree.

pub mod ast;
mod attrs;
mod auto_ids;
mod balance;
mod block;
mod entity;
mod inline;
mod line;
mod python;
mod render;
mod smart;
mod tagfilter;

pub use ast::{
    Align, Attr, Block, DefinitionItem, Document, Footnote, Inline, LinkRef, ListItem, TableCell,
    TableCellContent, TableCellData, TableRow, TableRowData,
};
pub use balance::balance_fragment;
pub use block::BlockSpan;
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
    pub balance: bool,
    pub auto_ids: bool,
    pub smart: bool,
    pub max_inline_depth: usize,
    pub max_block_depth: usize,
    pub max_link_paren_depth: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            math: MathMode::Brackets,
            tagfilter: false,
            balance: false,
            auto_ids: true,
            smart: false,
            max_inline_depth: 64,
            max_block_depth: 128,
            max_link_paren_depth: 32,
        }
    }
}

pub fn parse(src: &str, options: &Options) -> Document {
    let mut doc = block::parse_document(src, options);
    if options.auto_ids {
        auto_ids::assign(&mut doc);
    }
    if options.smart {
        smart::apply(&mut doc);
    }
    doc
}
pub fn block_spans(src: &str, options: &Options) -> Vec<BlockSpan> {
    block::parse_block_spans(src, options)
}
pub fn to_xhtml(src: &str, options: &Options) -> String {
    let out = render::to_xhtml_document(&parse(src, options));
    if options.balance {
        balance::balance_fragment(&out)
    } else {
        out
    }
}
