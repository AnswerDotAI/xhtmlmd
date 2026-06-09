# Dialect choices

When Markdown extensions disagree, this crate chooses the behavior closest to Pandoc unless that would conflict with GFM for a GFM-named feature.

- Pipe tables follow PHP Markdown Extra/Pandoc/GFM alignment markers. Header rows are required.
- Fenced divs follow Pandoc: an opening fence has at least three colons and attributes or a single class word; a closing fence is a colon-only line of at least three colons.
- Attribute syntax is kramdown/Pandoc-compatible: `#id`, `.class`, `key="value"`, ALDs, block IALs, and span IALs. Key/value pairs override earlier keys; classes accumulate.
- Definition lists follow PHP Markdown Extra/Pandoc: one-line terms with one or more `:` or `~` definitions.
- Footnotes follow Pandoc/kramdown label rules and render as XHTML endnotes with backlinks.
- `<tag markdown="1">` parses block Markdown inside the balanced tag. `markdown="span"` parses inline content into a single paragraph child.
- Math modes are explicit. `MathMode::Off` treats TeX delimiters as text. `MathMode::Brackets` recognizes only `\(...\)` and `\[...\]`. `MathMode::Dollars` additionally recognizes `$...$` and `$$...$$` with Pandoc's guard against currency-like spans.
