# xhtml-md-parser

A Rust Markdown parser and XHTML renderer.

The parser is deliberately tree-oriented: it preserves structure and attributes needed for XHTML output, but it does not aim to round-trip source text. The dialect is CommonMark/GFM for the core and GFM features, with Pandoc-leaning choices where extension families disagree.

## Implemented syntax

- Core block syntax: paragraphs, ATX/setext headings, thematic breaks, block quotes, ordered/unordered lists, indented code, raw HTML, link reference definitions.
- GFM: pipe tables with alignment, task lists, strikethrough, angle and bare autolinks, plus opt-in tagfiltering.
- Code: backtick/tilde fenced code blocks, info strings, and Pandoc-style code attributes.
- HTML-in-Markdown: `<div markdown="1">`, `markdown="block"`, and `markdown="span"` containers.
- Math: three modes: `off`, `brackets` for `\(...\)` and `\[...\]`, and `dollars` for those plus `$...$` and `$$...$$` using Pandoc's non-space/digit dollar rules.
- Attributes and inline spans: Pandoc/kramdown-style `{#id .class key="value"}`, block IALs `{: ...}`, span IALs, ALDs such as `{:note: #id .class}` with references, superscript `^x^`, and highlight `==x==`.
- Definition lists: PHP Markdown Extra/Pandoc-style `Term` followed by `: definition` or `~ definition`.
- Footnotes: `[^id]` references to defined `[^id]:` definitions with indented continuation blocks.
- Fenced divs: Pandoc/Quarto/Djot-style `:::` containers with attributes or a single class word.

## Parsing strategy

The implementation is moving toward the CommonMark parsing architecture: track visual columns and byte offsets for each line, determine block structure with an arena-backed open-container stack, collect link reference definitions, then finalize raw inline text with the completed reference table. The stack has typed nodes for block quotes, lists, paragraphs/setext candidates, fenced and indented code, raw HTML, GFM table candidates, math, footnote definitions, definition lists, fenced divs, and markdown-in-HTML containers. Inlines are scanned into atoms, bracket openers, and delimiter runs; links/images/spans resolve through the bracket stack, while emphasis/strong/strikethrough resolve through the delimiter stack. Potentially explosive constructs have explicit bounds: inline nesting, block/container nesting, link label length, and link parenthesis nesting.

The link parser uses raw reference-label scanning, bounded parenthesis nesting, bounded link labels, URI escaping for rendered href/src attributes, and a plain-text fast path for inputs with no possible inline constructs. This is intended to keep adversarial inputs such as deeply nested brackets, long blockquote runs, repeated `![[]()`, and unclosed comments in predictable time.

Raw HTML is preserved by default. `Options::default().extensions.tagfilter` is `false`; enabling it applies GFM-style filtering for tags such as `script`, `style`, `xmp`, and `textarea`. This is compatibility and defense-in-depth, not a replacement for sanitizing untrusted rendered HTML.

## Usage

```bash
cargo run --release -- --math=dollars input.md > out.xhtml
cat input.md | cargo run --release -- --math=brackets
```

Library usage:

```rust
use xhtml_md_parser::{to_xhtml, Options, MathMode};

let mut options = Options::default();
options.math = MathMode::Dollars;
let html = to_xhtml("# Hello", &options);
```

## Tests

The tree includes a focused extension fixture in `tests/fixtures/`, pathological smoke tests in `tests/pathological.rs`, copied cmark-gfm specs, and a notebook-derived spec from `meta/mf.ipynb`.

```bash
cargo test
cargo test --test conformance -- --nocapture
```

The spec harness has a per-example timeout and supports `XHTML_MD_CONFORMANCE_SECTION`, `XHTML_MD_CONFORMANCE_EXAMPLE`, `XHTML_MD_CONFORMANCE_LIMIT`, and `XHTML_MD_CONFORMANCE_TRACE` for narrowing failures.
