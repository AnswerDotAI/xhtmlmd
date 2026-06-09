# xhtml-md-parser

A dependency-free Rust Markdown parser and XHTML renderer.

The parser is deliberately tree-oriented: it preserves structure and attributes needed for XHTML output, but it does not aim to round-trip source text. The dialect is CommonMark/GFM for the core and GFM features, with Pandoc-leaning choices where extension families disagree.

## Implemented syntax

- Core block syntax: paragraphs, ATX/setext headings, thematic breaks, block quotes, ordered/unordered lists, indented code, raw HTML, link reference definitions.
- GFM: pipe tables with alignment, task lists, strikethrough, angle and bare autolinks.
- Code: backtick/tilde fenced code blocks, info strings, and Pandoc-style code attributes.
- HTML-in-Markdown: `<div markdown="1">`, `markdown="block"`, and `markdown="span"` containers.
- Math: three modes: `off`, `brackets` for `\(...\)` and `\[...\]`, and `dollars` for those plus `$...$` and `$$...$$` using Pandoc's non-space/digit dollar rules.
- Attributes: Pandoc/kramdown-style `{#id .class key="value"}`, block IALs `{: ...}`, span IALs, and ALDs such as `{:note: #id .class}` with references.
- Definition lists: PHP Markdown Extra/Pandoc-style `Term` followed by `: definition` or `~ definition`.
- Footnotes: `[^id]` references and `[^id]:` definitions with indented continuation blocks.
- Fenced divs: Pandoc/Quarto/Djot-style `:::` containers with attributes or a single class word.

## Parsing strategy

The implementation follows the CommonMark parsing architecture: first determine block structure in a single line-oriented pass, then parse inline content with the completed reference-definition table. Inlines are scanned without regex backtracking. Potentially explosive constructs have explicit bounds: inline nesting, block/container nesting, and link parenthesis nesting.

The link parser uses bounded parenthesis nesting, bracket scans that consume failed bracketed groups as literal text, and suffix-failure flags for repeated unmatched emphasis delimiters. This is intended to keep adversarial inputs such as deeply nested brackets, long blockquote runs, repeated `![[]()`, and unclosed comments in predictable time.

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

The tree includes a focused fixture suite in `tests/fixtures/`, pathological smoke tests in `tests/pathological.rs`, and the uploaded parser-test-source guide copied into `tests/source/markdowntests.md`.

The environment used to generate this artifact did not include `rustc` or `cargo`, so the suite is written but was not executed here.
