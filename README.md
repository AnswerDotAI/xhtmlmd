# xhtmlmd

A Rust Markdown parser and XHTML renderer.

The parser is tree-oriented. It preserves the structure and attributes needed for XHTML output, but it does not try to round-trip source text. The dialect is CommonMark/GFM for the core and GFM features, with Pandoc-leaning choices where extension families disagree.

xhtmlmd is largely implemented using AI, except for the tests. The tests are largely adapted from [`cmark-gfm`](https://github.com/github/cmark-gfm), [PHP Markdown Extra](https://github.com/michelf/php-markdown), [kramdown](https://github.com/gettalong/kramdown), [Pandoc](https://github.com/jgm/pandoc), and [Mistlefoot](https://github.com/AnswerDotAI/mistlefoot/). Credit for xhtmlmd really belongs to the authors of these tests, and of the CommonMark docs, which is where the hard work was done.

## Implemented syntax

- Core block syntax: paragraphs, ATX/setext headings, thematic breaks, block quotes, ordered/unordered lists, indented code, raw HTML, link reference definitions.
- Tables: GFM/PHP Extra pipe tables with alignment, and Pandoc grid tables with alignment, headerless tables, block cell content, row spans, column spans, footers, and pipe-table column widths from separator dash counts.
- GFM: task lists, `~~x~~` strikethrough, angle and bare autolinks, plus opt-in tagfiltering.
- Code: backtick/tilde fenced code blocks, info strings, and Pandoc-style code attributes.
- HTML-in-Markdown: block containers opened with `markdown="1"`; the control attribute is stripped, indented code blocks are disabled inside the container, and fenced code is the code-block syntax there.
- Math: four modes: `brackets` for `\(...\)`, `\[...\]`, and `$$...$$`, `dollars` for those plus `$...$` using Pandoc's non-space/digit dollar rules, `on` to preserve `\(...\)` and `\[...\]` delimiters for client-side renderers such as KaTeX, and `off`. Brackets mode is the default.
- Attributes and inline spans: Pandoc/kramdown-style `{#id .class key="value"}`, block IALs `{: ...}`, span IALs, ALDs such as `{:note: #id .class}` with references, superscript `^x^`, subscript `~x~`, and highlight `==x==`.
- Definition lists: PHP Markdown Extra/Pandoc-style `Term` followed by `: definition` or `~ definition`.
- Footnotes: `[^id]` references to defined `[^id]:` definitions with indented continuation blocks.
- Abbreviations: `*[HTML]: Hyper Text Markup Language` definitions render matching text as `<abbr>`.
- Fenced divs: Pandoc/Quarto/Djot-style `:::` containers with attributes or a single class word.

## Attributes

A braced group is an attribute list only when it starts with `:`, `#`, `.`, or a `key=value` pair. Anything else in braces is ordinary text, so prose like `use {braces} freely` keeps its content. The marker forms follow Pandoc: `{#id .class key="value"}`. The colon form follows kramdown: `{:note}` and `{: note}` apply the attribute definition named `note`, and an unknown name in a colon-marked list is ignored while the list itself is still consumed.

ALDs (attribute list definitions) are kramdown's named bundles. `{:note: #id .class}` on its own line defines `note`; a reference resolves either as a colon-marked list (`{:note}`) or as a bare token inside a list already recognized by its markers (`{.x note}`).

Attribute lists attach to:

- Headings, ATX and setext: `# Head {#h}`.
- Paragraphs: a trailing list at the end of the last line.
- Fenced code: in the info string, `python {.numberLines}` after the opening fence.
- Fenced divs: in the `:::` opener.
- Link reference definitions: `[r]: /url "title" {.external}` applies the attributes to every link resolved through that reference.
- Any block, via a standalone IAL line `{: ...}`: it modifies the preceding block, including directly after the last row of a table; with no preceding block it applies to the next one.
- Inline constructs, when the list follows immediately with no space: spans `[x]{.c}`, links, images, code spans, emphasis, strong, strikethrough, superscript, subscript, highlight, and math.

Raw HTML blocks take no attribute lists; write attributes in the HTML itself.


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

### Table column widths

Pipe tables can state relative column widths through the separator row: `|------|--|` renders as a `<colgroup>` giving the columns 75% and 25%. This is on by default; pass `table_widths=False` (CLI `--no-table-widths`) to turn it off.

```python
html = to_xhtml("| a | b |\n|------|--|\n| x | y |\n")
```

Each width is the separator cell's character count (alignment colons included) over the row's total, truncated to a whole percent. This matches what Pandoc's HTML writer emits, and Pandoc's HTML reader turns it back into column widths. A separator row whose cells are all the same length sets no widths, so the browser lays the table out as usual.

### Markdown rewriting

`rewrite` changes recognized Markdown constructs without regenerating the rest of the document. A callback returns `None` to leave a construct alone, a string to replace the whole construct, or a dict to replace one of its named fields.

This converts inline dollar math to bracket math:

```python
from xhtmlmd import rewrite

def bracket_math(node):
    if node["delimiter"] != "$": return None
    return rf"\({node['tex']}\)"

markdown = rewrite(markdown, {"math_inline": bracket_math}, math="dollars")
```

An image callback can save a data URL and replace only its destination. The alt text, title, attributes, and original spacing are preserved.

```python
from base64 import b64decode
from pathlib import Path
from xhtmlmd import rewrite

def save_image(node):
    if not node["url"].startswith("data:image/png;base64,"): return None
    path = Path("images/plot.png")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(b64decode(node["url"].split(",", 1)[1]))
    return {"url": path.as_posix()}

markdown = rewrite(markdown, {"image": save_image})
```

Callbacks run in source order. Their edits are checked first and then applied from the end of the document, so an early replacement cannot invalidate a later source position. Exceptions from callbacks are passed through unchanged.

Every callback node is a dict with these common fields:

- `type`: callback name, currently `image` or `math_inline`.
- `source`: the exact source text for the construct.
- `start`, `end`: half-open character offsets into the original Python string.

An `image` node has:

- `form`: currently always `inline`.
- `alt`: plain alt text.
- `url`: the decoded image destination.
- `title`: decoded title text, or `None`.

An image callback may return `{"url": "new destination"}`. Other image fields are read-only. Reference-style images such as `![alt][id]` are not callback targets.

A `math_inline` node has:

- `delimiter`: `$`, `$$`, `\(`, or `\[`.
- `tex`: content without delimiters.
- `display`: `True` for `$$` and `\[`, otherwise `False`.

A math callback may return `{"tex": "new TeX"}` to preserve the delimiters, or a string to replace the entire construct. Dollar math is recognized only with `math="dollars"`, using the same dollar rules as rendering.

Rewriting is confined to inline-capable prose regions. Inline code, fenced and indented code blocks, raw HTML blocks, block math, link reference definitions, and grid tables are left untouched. Inline images and math inside paragraphs, headings, lists, block quotes, definition bodies, footnotes, and pipe tables are supported.

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
    html = mathml.convert_with_local_state(node["tex"], displaystyle=node["type"] == "math_block")
    return html + ("\n" if node["type"] == "math_block" else "")

html = to_xhtml(markdown, callbacks={"math_inline": render_math, "math_block": render_math})
```

### Block spans

`blocks` reports where each top-level block sits in the source, so callers can split a document into per-block source slices without regenerating Markdown from a tree. Each dict has `type` (the callback names above, plus `link_ref`, `abbr_def`, `attr_def`, and `footnote_def`) and half-open 0-based `start`/`end` line indices; code and math blocks also carry their inner `text`, and fences carry `info`/`lang`.

```python
from xhtmlmd import blocks

src = open("input.md").read()
lines = src.split("\n")
for b in blocks(src):
    print(b["type"], "\n".join(lines[b["start"]:b["end"]]))
```

Command-line usage (the `xhtmlmd` script is installed with the package):

```bash
xhtmlmd input.md > out.xhtml
cat input.md | xhtmlmd --math=dollars
```

## Parsing strategy

The parser uses the two-phase strategy described in the [CommonMark parsing-strategy appendix](https://spec.commonmark.org/0.31.2/#appendix-a-parsing-strategy): first build the block tree and collect link reference definitions, then parse raw inline text with the completed reference table. It tracks visual columns and byte offsets for each line and builds blocks with an arena-backed open-container stack. The stack has typed nodes for block quotes, lists, paragraphs/setext candidates, fenced and indented code, raw HTML, table candidates, grid tables, math, footnote definitions, definition lists, fenced divs, and markdown-in-HTML containers. Inlines are scanned into atoms, bracket openers, and delimiter runs; links/images/spans resolve through the bracket stack, while emphasis/strong/strikethrough resolve through the delimiter stack. Inputs that can otherwise explode have explicit bounds: inline nesting, block/container nesting, link label length, and link parenthesis nesting.

The link parser uses raw reference-label scanning, bounded parenthesis nesting, bounded link labels, URI escaping for rendered href/src attributes, and a plain-text fast path for inputs with no possible inline constructs. This keeps adversarial inputs such as deeply nested brackets, long blockquote runs, repeated `![[]()`, and unclosed comments in predictable time.

Raw HTML is preserved by default. Supported raw HTML container tags such as `div`, `section`, `table`, `svg`, `math`, and custom elements stay open across blank lines until their matching close tag, with same-tag nesting counted; void and self-closing tags do not open balanced containers. Markdown inside raw HTML remains raw unless the open tag that starts the Markdown block uses `markdown="1"`; this crate does not recursively look for markdown controls inside otherwise-raw HTML. `Options::default().tagfilter` is `false`; enabling it applies GFM-style filtering for tags such as `script`, `style`, `xmp`, and `textarea`. This is compatibility and extra protection, not a replacement for sanitizing untrusted rendered HTML.

Raw HTML passthrough means unbalanced source HTML produces an unbalanced fragment, exactly as CommonMark specifies. The opt-in `balance` option (`Options::default().balance` is `false`; `--balance` on the CLI) restores well-formedness after rendering: unclosed elements are closed at the end of the fragment, stray closing tags are dropped, a closing tag that skips over open elements closes them first, void elements are rewritten to self-closing form, and rawtext elements such as `script` are copied verbatim to their real close. It deliberately does not apply HTML5 implied-end-tag rules (no `<p>` auto-close) or rewrite attributes.

## Tests

```bash
maturin develop && pytest -q
```

The spec-conformance suite is `tests/test_conformance.py`: it renders the fixtures under `tests/source/` and compares normalized HTML trees. Run just that file with `pytest tests/test_conformance.py -v` to see per-example ids.
