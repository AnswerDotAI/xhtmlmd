# xhtmlmd

A Rust Markdown parser and XHTML renderer.

The parser is tree-oriented. It preserves the structure and attributes needed for XHTML output, but it does not try to round-trip source text. The dialect is CommonMark/GFM for the core and GFM features, with Pandoc-leaning choices where extension families disagree.

xhtmlmd is largely implemented using AI, except for the tests. The tests are largely adapted from [`cmark-gfm`](https://github.com/github/cmark-gfm), [PHP Markdown Extra](https://github.com/michelf/php-markdown), [kramdown](https://github.com/gettalong/kramdown), and [Mistlefoot](https://github.com/AnswerDotAI/mistlefoot/). Credit for xhtmlmd really belongs to the authors of these tests, and of the CommonMark docs, which is where the hard work was done.

## Implemented syntax

- Core block syntax: paragraphs, ATX/setext headings, thematic breaks, block quotes, ordered/unordered lists, indented code, raw HTML, link reference definitions.
- GFM: pipe tables with alignment, task lists, `~~x~~` strikethrough, angle and bare autolinks, plus opt-in tagfiltering.
- Code: backtick/tilde fenced code blocks, info strings, and Pandoc-style code attributes.
- HTML-in-Markdown: block containers opened with `markdown="1"`; the control attribute is stripped, indented code blocks are disabled inside the container, and fenced code is the code-block syntax there.
- Math: four modes: `brackets` for `\(...\)` and `\[...\]`, `dollars` for those plus `$...$` and `$$...$$` using Pandoc's non-space/digit dollar rules, `on` to preserve `\(...\)` and `\[...\]` delimiters for client-side renderers such as KaTeX, and `off`. Brackets mode is the default.
- Attributes and inline spans: Pandoc/kramdown-style `{#id .class key="value"}`, block IALs `{: ...}`, span IALs, ALDs such as `{:note: #id .class}` with references, superscript `^x^`, subscript `~x~`, and highlight `==x==`.
- Definition lists: PHP Markdown Extra/Pandoc-style `Term` followed by `: definition` or `~ definition`.
- Footnotes: `[^id]` references to defined `[^id]:` definitions with indented continuation blocks.
- Abbreviations: `*[HTML]: Hyper Text Markup Language` definitions render matching text as `<abbr>`.
- Fenced divs: Pandoc/Quarto/Djot-style `:::` containers with attributes or a single class word.

## Usage

Install via pip to get both the Python API and the native `xhtmlmd` CLI:

```bash
pip install xhtmlmd
```

The CLI reads Markdown from stdin or from an optional file path and writes an XHTML fragment to stdout:

```bash
echo '# Hello' | xhtmlmd
xhtmlmd input.md > out.xhtml
xhtmlmd --math=on input.md > out.xhtml
xhtmlmd --math=dollars input.md > out.xhtml
```

Python API:

```python
from xhtmlmd import to_xhtml

html = to_xhtml(r"\(x^2\)")
html_for_katex = to_xhtml(r"\(x^2\)", math="on")
html_with_dollars = to_xhtml("$x$", math="dollars")
```

### Callbacks

Python callers can override rendered nodes with callbacks. Each callback receives a node dict and the default XHTML for that node. Return `None` to keep the default, or return replacement XHTML.

Callback names:

- Blocks: `paragraph`, `heading`, `block_quote`, `list`, `definition_list`, `code_block`, `html_block`, `html_container`, `thematic_break`, `table`, `div`, `math_block`
- Inlines: `text`, `soft_break`, `hard_break`, `emph`, `strong`, `strike`, `superscript`, `subscript`, `highlight`, `code`, `link`, `image`, `autolink`, `abbr`, `html_inline`, `math_inline`, `footnote_ref`, `span`

```python
from fastpylight import highlight
from xhtmlmd import to_xhtml

def highlight_code(node, default_html):
    if node["lang"] != "python": return None
    return highlight(node["text"], node["lang"]) + "\n"

html = to_xhtml(markdown, callbacks={"code_block": highlight_code})
```

Callbacks can also render bracket math as MathML:

```python
from math_core import LatexToMathML
from xhtmlmd import to_xhtml

mathml = LatexToMathML()

def render_math(node, default_html):
    html = mathml.convert_with_local_counter(node["tex"], displaystyle=node["type"] == "math_block")
    return html + ("\n" if node["type"] == "math_block" else "")

html = to_xhtml(markdown, callbacks={"math_inline": render_math, "math_block": render_math})
```

Rust/source usage:

```bash
cargo run --release -- input.md > out.xhtml
cat input.md | cargo run --release -- --math=dollars
```

Library usage:

```rust
use xhtmlmd::{to_xhtml, Options, MathMode};

let mut options = Options::default();
options.math = MathMode::Dollars;
let html = to_xhtml("$x$", &options);
```

## Parsing strategy

The parser uses the two-phase strategy described in the [CommonMark parsing-strategy appendix](https://spec.commonmark.org/0.31.2/#appendix-a-parsing-strategy): first build the block tree and collect link reference definitions, then parse raw inline text with the completed reference table. It tracks visual columns and byte offsets for each line and builds blocks with an arena-backed open-container stack. The stack has typed nodes for block quotes, lists, paragraphs/setext candidates, fenced and indented code, raw HTML, GFM table candidates, math, footnote definitions, definition lists, fenced divs, and markdown-in-HTML containers. Inlines are scanned into atoms, bracket openers, and delimiter runs; links/images/spans resolve through the bracket stack, while emphasis/strong/strikethrough resolve through the delimiter stack. Inputs that can otherwise explode have explicit bounds: inline nesting, block/container nesting, link label length, and link parenthesis nesting.

The link parser uses raw reference-label scanning, bounded parenthesis nesting, bounded link labels, URI escaping for rendered href/src attributes, and a plain-text fast path for inputs with no possible inline constructs. This keeps adversarial inputs such as deeply nested brackets, long blockquote runs, repeated `![[]()`, and unclosed comments in predictable time.

Raw HTML is preserved by default. Supported raw HTML container tags such as `div`, `section`, `table`, `svg`, `math`, and custom elements stay open across blank lines until their matching close tag, with same-tag nesting counted; void and self-closing tags do not open balanced containers. Markdown inside raw HTML remains raw unless the open tag that starts the Markdown block uses `markdown="1"`; this crate does not recursively look for markdown controls inside otherwise-raw HTML. `Options::default().tagfilter` is `false`; enabling it applies GFM-style filtering for tags such as `script`, `style`, `xmp`, and `textarea`. This is compatibility and extra protection, not a replacement for sanitizing untrusted rendered HTML.

## Tests

```bash
ship-rs-test
```

Use `cargo test --test conformance -- --nocapture` when you want the per-section conformance report. The harness supports `XHTML_MD_CONFORMANCE_SECTION`, `XHTML_MD_CONFORMANCE_EXAMPLE`, `XHTML_MD_CONFORMANCE_LIMIT`, and `XHTML_MD_CONFORMANCE_TRACE` for narrowing failures.
