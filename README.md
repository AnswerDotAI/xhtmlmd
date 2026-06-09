# xhtmlmd

A Rust Markdown parser and XHTML renderer.

The parser is deliberately tree-oriented: it preserves structure and attributes needed for XHTML output, but it does not aim to round-trip source text. The dialect is CommonMark/GFM for the core and GFM features, with Pandoc-leaning choices where extension families disagree.

## Implemented syntax

- Core block syntax: paragraphs, ATX/setext headings, thematic breaks, block quotes, ordered/unordered lists, indented code, raw HTML, link reference definitions.
- GFM: pipe tables with alignment, task lists, strikethrough, angle and bare autolinks, plus opt-in tagfiltering.
- Code: backtick/tilde fenced code blocks, info strings, and Pandoc-style code attributes.
- HTML-in-Markdown: block containers opened with `markdown="1"`; the control attribute is stripped, indented code blocks are disabled inside the container, and fenced code is the code-block syntax there.
- Math: three modes: `off`, `brackets` for `\(...\)` and `\[...\]`, and `dollars` for those plus `$...$` and `$$...$$` using Pandoc's non-space/digit dollar rules.
- Attributes and inline spans: Pandoc/kramdown-style `{#id .class key="value"}`, block IALs `{: ...}`, span IALs, ALDs such as `{:note: #id .class}` with references, superscript `^x^`, and highlight `==x==`.
- Definition lists: PHP Markdown Extra/Pandoc-style `Term` followed by `: definition` or `~ definition`.
- Footnotes: `[^id]` references to defined `[^id]:` definitions with indented continuation blocks.
- Abbreviations: `*[HTML]: Hyper Text Markup Language` definitions render matching text as `<abbr>`.
- Fenced divs: Pandoc/Quarto/Djot-style `:::` containers with attributes or a single class word.

## Parsing strategy

The implementation is moving toward the CommonMark parsing architecture: track visual columns and byte offsets for each line, determine block structure with an arena-backed open-container stack, collect link reference definitions, then finalize raw inline text with the completed reference table. The stack has typed nodes for block quotes, lists, paragraphs/setext candidates, fenced and indented code, raw HTML, GFM table candidates, math, footnote definitions, definition lists, fenced divs, and markdown-in-HTML containers. Inlines are scanned into atoms, bracket openers, and delimiter runs; links/images/spans resolve through the bracket stack, while emphasis/strong/strikethrough resolve through the delimiter stack. Potentially explosive constructs have explicit bounds: inline nesting, block/container nesting, link label length, and link parenthesis nesting.

The link parser uses raw reference-label scanning, bounded parenthesis nesting, bounded link labels, URI escaping for rendered href/src attributes, and a plain-text fast path for inputs with no possible inline constructs. This is intended to keep adversarial inputs such as deeply nested brackets, long blockquote runs, repeated `![[]()`, and unclosed comments in predictable time.

Raw HTML is preserved by default. Supported raw HTML container tags such as `div`, `section`, `table`, `svg`, `math`, and custom elements stay open across blank lines until their matching close tag, with same-tag nesting counted; void and self-closing tags do not open balanced containers. Markdown inside raw HTML remains raw unless the open tag that starts the Markdown block uses `markdown="1"`; this crate does not recursively look for markdown controls inside otherwise-raw HTML. `Options::default().tagfilter` is `false`; enabling it applies GFM-style filtering for tags such as `script`, `style`, `xmp`, and `textarea`. This is compatibility and defense-in-depth, not a replacement for sanitizing untrusted rendered HTML.

## Usage

Install via pip to get both the Python API and the native `xhtmlmd` CLI:

```bash
pip install xhtmlmd
```

The CLI reads Markdown from stdin or from an optional file path and writes an XHTML fragment to stdout:

```bash
echo '# Hello' | xhtmlmd
xhtmlmd --math=brackets input.md > out.xhtml
```

Python API:

```python
from xhtmlmd import to_xhtml

html = to_xhtml("# Hello", math="dollars")
```

Rust/source usage:

```bash
cargo run --release -- --math=dollars input.md > out.xhtml
cat input.md | cargo run --release -- --math=brackets
```

Library usage:

```rust
use xhtmlmd::{to_xhtml, Options, MathMode};

let mut options = Options::default();
options.math = MathMode::Dollars;
let html = to_xhtml("# Hello", &options);
```

## Tests

The tree includes a focused extension fixture in `tests/fixtures/`, pathological smoke tests in `tests/pathological.rs`, copied cmark-gfm specs, and a notebook-derived spec from `meta/mf.ipynb`.

```bash
tools/test.sh
cargo test --test conformance -- --nocapture
maturin develop
pytest tests/test_python.py
```

The spec harness has a per-example timeout and supports `XHTML_MD_CONFORMANCE_SECTION`, `XHTML_MD_CONFORMANCE_EXAMPLE`, `XHTML_MD_CONFORMANCE_LIMIT`, and `XHTML_MD_CONFORMANCE_TRACE` for narrowing failures.
