use std::panic::{catch_unwind, AssertUnwindSafe};

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::ast::{Attr, Block, Document, Inline, TableCellContent};
use crate::render::{code_block_open, plain, to_xhtml_document, to_xhtml_inlines, CODE_BLOCK_CLOSE};
use crate::{MathMode, Options};

#[pyfunction]
#[pyo3(signature = (
    markdown,
    *,
    math = "brackets",
    tagfilter = false,
    balance = false,
    underline = false,
    callbacks = None,
    max_inline_depth = None,
    max_block_depth = None,
    max_link_paren_depth = None
))]
fn to_xhtml(
    markdown: &str,
    math: &str,
    tagfilter: bool,
    balance: bool,
    underline: bool,
    callbacks: Option<Bound<'_, PyDict>>,
    max_inline_depth: Option<usize>,
    max_block_depth: Option<usize>,
    max_link_paren_depth: Option<usize>,
) -> PyResult<String> {
    let mut options = Options {
        math: parse_math_mode(math)?,
        tagfilter,
        balance,
        underline,
        ..Options::default()
    };
    if let Some(depth) = max_inline_depth {
        options.max_inline_depth = depth;
    }
    if let Some(depth) = max_block_depth {
        options.max_block_depth = depth;
    }
    if let Some(depth) = max_link_paren_depth {
        options.max_link_paren_depth = depth;
    }
    if let Some(callbacks) = callbacks {
        let mut doc = guard("parsing markdown", || crate::parse(markdown, &options))?;
        apply_callbacks(&mut doc, &callbacks)?;
        let out = guard("rendering markdown", || to_xhtml_document(&doc))?;
        if balance {
            guard("balancing output", || crate::balance_fragment(&out))
        } else {
            Ok(out)
        }
    } else {
        guard("rendering markdown", || crate::to_xhtml(markdown, &options))
    }
}

/// Run a panic-prone pure-Rust render step, converting any panic into a clean
/// `RuntimeError` rather than aborting the interpreter or surfacing pyo3's
/// `BaseException`-derived `PanicException`. The default panic hook still logs
/// the panic location to stderr, which is what you want when reporting the bug.
fn guard<T>(what: &str, f: impl FnOnce() -> T) -> PyResult<T> {
    catch_unwind(AssertUnwindSafe(f)).map_err(|_| {
        PyRuntimeError::new_err(format!(
            "internal error in xhtmlmd while {what} (this is a bug, please report it)"
        ))
    })
}

#[pyfunction]
#[pyo3(signature = (markdown, *, math = "brackets"))]
fn blocks(py: Python<'_>, markdown: &str, math: &str) -> PyResult<Vec<Py<PyDict>>> {
    let options = Options {
        math: parse_math_mode(math)?,
        ..Options::default()
    };
    let spans = guard("parsing markdown", || crate::block_spans(markdown, &options))?;
    spans
        .into_iter()
        .map(|span| {
            let d = PyDict::new(py);
            d.set_item("type", span.kind)?;
            d.set_item("start", span.start)?;
            d.set_item("end", span.end)?;
            if let Some(info) = span.info {
                d.set_item("info", info)?;
            }
            if let Some(lang) = span.lang {
                d.set_item("lang", lang)?;
            }
            if let Some(text) = span.text {
                d.set_item("text", text)?;
            }
            Ok(d.unbind())
        })
        .collect()
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(to_xhtml, m)?)?;
    m.add_function(wrap_pyfunction!(blocks, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}

fn parse_math_mode(mode: &str) -> PyResult<MathMode> {
    match mode {
        "off" => Ok(MathMode::Off),
        "on" => Ok(MathMode::On),
        "brackets" => Ok(MathMode::Brackets),
        "dollars" => Ok(MathMode::Dollars),
        _ => Err(PyValueError::new_err(
            "math must be 'off', 'on', 'brackets', or 'dollars'",
        )),
    }
}

fn apply_callbacks(doc: &mut Document, callbacks: &Bound<'_, PyDict>) -> PyResult<()> {
    transform_blocks(&mut doc.blocks, callbacks)?;
    for footnote in &mut doc.footnotes {
        transform_blocks(&mut footnote.blocks, callbacks)?;
    }
    Ok(())
}

fn transform_blocks(blocks: &mut [Block], callbacks: &Bound<'_, PyDict>) -> PyResult<()> {
    for block in blocks {
        transform_block(block, callbacks)?;
    }
    Ok(())
}

fn transform_block(block: &mut Block, callbacks: &Bound<'_, PyDict>) -> PyResult<()> {
    match block {
        Block::Paragraph { children, .. } | Block::Heading { children, .. } => {
            transform_inlines(children, callbacks)?;
        }
        Block::BlockQuote { children, .. }
        | Block::HtmlContainer { children, .. }
        | Block::Div { children, .. } => {
            transform_blocks(children, callbacks)?;
        }
        Block::List { items, .. } => {
            for item in items {
                transform_blocks(&mut item.blocks, callbacks)?;
            }
        }
        Block::DefinitionList { items, .. } => {
            for item in items {
                for term in &mut item.terms {
                    transform_inlines(term, callbacks)?;
                }
                for def in &mut item.definitions {
                    transform_blocks(&mut def.blocks, callbacks)?;
                }
            }
        }
        Block::Table {
            head, rows, foot, ..
        } => {
            for row in head.iter_mut().chain(rows).chain(foot) {
                for cell in &mut row.cells {
                    match &mut cell.content {
                        TableCellContent::Inline(items) => transform_inlines(items, callbacks)?,
                        TableCellContent::Blocks(blocks) => transform_blocks(blocks, callbacks)?,
                    }
                }
            }
        }
        Block::CodeBlock { .. }
        | Block::Html { .. }
        | Block::ThematicBreak { .. }
        | Block::Math { .. } => {}
    }
    if let Some(html) = call_block_callback(block, callbacks)? {
        *block = Block::Html { raw: html };
    }
    Ok(())
}

fn transform_inlines(items: &mut [Inline], callbacks: &Bound<'_, PyDict>) -> PyResult<()> {
    for item in items {
        transform_inline(item, callbacks)?;
    }
    Ok(())
}

fn transform_inline(item: &mut Inline, callbacks: &Bound<'_, PyDict>) -> PyResult<()> {
    match item {
        Inline::Emph { children, .. }
        | Inline::Strong { children, .. }
        | Inline::Underline { children, .. }
        | Inline::Strike { children, .. }
        | Inline::Highlight { children, .. }
        | Inline::Link { children, .. }
        | Inline::Span { children, .. } => transform_inlines(children, callbacks)?,
        Inline::Image { alt, .. } => transform_inlines(alt, callbacks)?,
        Inline::Text(_)
        | Inline::SoftBreak
        | Inline::HardBreak
        | Inline::Superscript { .. }
        | Inline::Subscript { .. }
        | Inline::Code { .. }
        | Inline::Autolink { .. }
        | Inline::Abbr { .. }
        | Inline::Html(_)
        | Inline::Math { .. }
        | Inline::FootnoteRef { .. } => {}
    }
    if let Some(html) = call_inline_callback(item, callbacks)? {
        *item = Inline::Html(html);
    }
    Ok(())
}

fn call_block_callback(block: &Block, callbacks: &Bound<'_, PyDict>) -> PyResult<Option<String>> {
    let kind = block_kind(block);
    let Some(callback) = callbacks.get_item(kind)? else {
        return Ok(None);
    };
    let node = block_node(callbacks.py(), block)?;
    let default_html = to_xhtml_document(&Document {
        blocks: vec![block.clone()],
        ..Document::default()
    });
    call_callback(callback, node, default_html)
}

fn call_inline_callback(item: &Inline, callbacks: &Bound<'_, PyDict>) -> PyResult<Option<String>> {
    let kind = inline_kind(item);
    let Some(callback) = callbacks.get_item(kind)? else {
        return Ok(None);
    };
    let node = inline_node(callbacks.py(), item)?;
    call_callback(callback, node, to_xhtml_inlines(std::slice::from_ref(item)))
}

fn call_callback(
    callback: Bound<'_, PyAny>,
    node: Bound<'_, PyDict>,
    default_html: String,
) -> PyResult<Option<String>> {
    let result = callback.call1((node, default_html))?;
    if result.is_none() {
        Ok(None)
    } else {
        result.extract::<String>().map(Some)
    }
}

fn block_kind(block: &Block) -> &'static str {
    match block {
        Block::Paragraph { .. } => "paragraph",
        Block::Heading { .. } => "heading",
        Block::BlockQuote { .. } => "block_quote",
        Block::List { .. } => "list",
        Block::DefinitionList { .. } => "definition_list",
        Block::CodeBlock { .. } => "code_block",
        Block::Html { .. } => "html_block",
        Block::HtmlContainer { .. } => "html_container",
        Block::ThematicBreak { .. } => "thematic_break",
        Block::Table { .. } => "table",
        Block::Div { .. } => "div",
        Block::Math { .. } => "math_block",
    }
}

fn inline_kind(item: &Inline) -> &'static str {
    match item {
        Inline::Text(_) => "text",
        Inline::SoftBreak => "soft_break",
        Inline::HardBreak => "hard_break",
        Inline::Emph { .. } => "emph",
        Inline::Strong { .. } => "strong",
        Inline::Underline { .. } => "underline",
        Inline::Strike { .. } => "strike",
        Inline::Superscript { .. } => "superscript",
        Inline::Subscript { .. } => "subscript",
        Inline::Highlight { .. } => "highlight",
        Inline::Code { .. } => "code",
        Inline::Link { .. } => "link",
        Inline::Image { .. } => "image",
        Inline::Autolink { .. } => "autolink",
        Inline::Abbr { .. } => "abbr",
        Inline::Html(_) => "html_inline",
        Inline::Math { .. } => "math_inline",
        Inline::FootnoteRef { .. } => "footnote_ref",
        Inline::Span { .. } => "span",
    }
}

fn block_node<'py>(py: Python<'py>, block: &Block) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    d.set_item("type", block_kind(block))?;
    match block {
        Block::Paragraph { attrs, children } => {
            set_attrs(&d, attrs)?;
            d.set_item("children", children.len())?;
        }
        Block::Heading {
            level,
            attrs,
            children,
        } => {
            d.set_item("level", *level)?;
            set_attrs(&d, attrs)?;
            d.set_item("children", children.len())?;
        }
        Block::BlockQuote { attrs, children } | Block::Div { attrs, children } => {
            set_attrs(&d, attrs)?;
            d.set_item("children", children.len())?;
        }
        Block::List {
            attrs,
            ordered,
            start,
            tight,
            items,
        } => {
            set_attrs(&d, attrs)?;
            d.set_item("ordered", *ordered)?;
            d.set_item("start", *start)?;
            d.set_item("tight", *tight)?;
            d.set_item("items", items.len())?;
        }
        Block::DefinitionList { attrs, items } => {
            set_attrs(&d, attrs)?;
            d.set_item("items", items.len())?;
        }
        Block::CodeBlock {
            attrs,
            info,
            lang,
            text,
        } => {
            set_attrs(&d, attrs)?;
            d.set_item("info", info)?;
            d.set_item("lang", lang.as_deref())?;
            d.set_item("text", text)?;
            d.set_item("open", code_block_open(attrs, lang.as_deref()))?;
            d.set_item("close", CODE_BLOCK_CLOSE)?;
        }
        Block::Html { raw } => {
            d.set_item("raw", raw)?;
        }
        Block::HtmlContainer {
            tag,
            attrs,
            children,
        } => {
            d.set_item("tag", tag)?;
            set_attrs(&d, attrs)?;
            d.set_item("children", children.len())?;
        }
        Block::ThematicBreak { attrs } => {
            set_attrs(&d, attrs)?;
        }
        Block::Table {
            attrs,
            aligns,
            head,
            rows,
            foot,
        } => {
            set_attrs(&d, attrs)?;
            d.set_item(
                "aligns",
                aligns.iter().map(ToString::to_string).collect::<Vec<_>>(),
            )?;
            d.set_item(
                "head_cells",
                head.iter().map(|row| row.cells.len()).sum::<usize>(),
            )?;
            d.set_item("head_rows", head.len())?;
            d.set_item("rows", rows.len())?;
            d.set_item("foot_rows", foot.len())?;
        }
        Block::Math {
            attrs,
            display,
            tex,
        } => {
            set_attrs(&d, attrs)?;
            d.set_item("display", *display)?;
            d.set_item("tex", tex)?;
        }
    }
    Ok(d)
}

fn inline_node<'py>(py: Python<'py>, item: &Inline) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    d.set_item("type", inline_kind(item))?;
    match item {
        Inline::Text(text) | Inline::Html(text) => {
            d.set_item("text", text)?;
        }
        Inline::SoftBreak | Inline::HardBreak => {}
        Inline::Emph { attrs, children }
        | Inline::Strong { attrs, children }
        | Inline::Underline { attrs, children }
        | Inline::Strike { attrs, children }
        | Inline::Highlight { attrs, children }
        | Inline::Span { attrs, children } => {
            set_attrs(&d, attrs)?;
            d.set_item("children", children.len())?;
        }
        Inline::Superscript { attrs, text }
        | Inline::Subscript { attrs, text }
        | Inline::Code { attrs, text } => {
            set_attrs(&d, attrs)?;
            d.set_item("text", text)?;
        }
        Inline::Link {
            attrs,
            children,
            url,
            title,
        } => {
            set_attrs(&d, attrs)?;
            d.set_item("children", children.len())?;
            d.set_item("url", url)?;
            d.set_item("title", title.as_deref())?;
        }
        Inline::Image {
            attrs,
            alt,
            url,
            title,
        } => {
            set_attrs(&d, attrs)?;
            d.set_item("alt", plain(alt))?;
            d.set_item("url", url)?;
            d.set_item("title", title.as_deref())?;
        }
        Inline::Autolink { url, text, email } => {
            d.set_item("url", url)?;
            d.set_item("text", text)?;
            d.set_item("email", *email)?;
        }
        Inline::Abbr { text, title } => {
            d.set_item("text", text)?;
            d.set_item("title", title)?;
        }
        Inline::Math {
            attrs,
            display,
            tex,
        } => {
            set_attrs(&d, attrs)?;
            d.set_item("display", *display)?;
            d.set_item("tex", tex)?;
        }
        Inline::FootnoteRef { label } => {
            d.set_item("label", label)?;
        }
    }
    Ok(d)
}

fn set_attrs(d: &Bound<'_, PyDict>, attrs: &Attr) -> PyResult<()> {
    d.set_item("attrs", attr_node(d.py(), attrs)?)?;
    Ok(())
}

fn attr_node<'py>(py: Python<'py>, attrs: &Attr) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    d.set_item("id", attrs.id.as_deref())?;
    d.set_item("classes", attrs.classes.clone())?;
    d.set_item("pairs", attrs.pairs.clone())?;
    Ok(d)
}
