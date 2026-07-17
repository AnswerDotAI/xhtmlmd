# Dialect choices

When Markdown extensions disagree, this crate chooses the behavior closest to Pandoc unless that would conflict with GFM for a GFM-named feature.

- Pipe tables follow PHP Markdown Extra/Pandoc/GFM alignment markers. Header rows are required.
- Pipe-table column widths are on by default (`table_widths=False` / `--no-table-widths` disables them) and follow Pandoc: each separator cell's length, alignment colons included, over the row's total, truncated to a whole percent and emitted as `<colgroup>` of `<col style="width: N%" />`. A separator row whose cells are all the same length sets no widths, and grid tables never do.
- Fenced divs follow Pandoc: an opening fence has at least three colons and attributes or a single class word; a closing fence is a colon-only line of at least three colons.
- Attribute syntax is kramdown/Pandoc-compatible: `#id`, `.class`, `key="value"`, ALDs, block IALs, and span IALs. A braced group is an attribute list only when it starts with `:`, `#`, `.`, or a `key=value` pair; bare words in braces stay literal text. Key/value pairs override earlier keys; classes accumulate. Trailing lists attach to emphasis, strong, and strikethrough as well as the bracket constructs, and an IAL line directly after a table attaches to the table.
- Definition lists follow PHP Markdown Extra/Pandoc: one-line terms with one or more `:` or `~` definitions.
- Footnotes follow Pandoc/kramdown label rules and render as XHTML endnotes with backlinks. The endnotes `<section>` has no leading `<hr>` (unlike cmark-gfm): separators are a styling concern, so add one with CSS if wanted.
- Inline `~~x~~` renders as strikethrough. Inline `~x~` renders as subscript, using the same no-whitespace rule as superscript `^x^`.
- `<tag markdown="1">` parses block Markdown inside the balanced tag. `markdown="span"` parses inline content into a single paragraph child.
- Raw HTML passes through unbalanced, per CommonMark. The opt-in `balance` option closes unclosed elements at the fragment end, drops stray closes, and self-closes void tags, without HTML5 implied-end-tag rules.
- Math defaults to `MathMode::Brackets`, which recognizes `\(...\)`, `\[...\]`, and `$$...$$`. `MathMode::Dollars` also recognizes `$...$` with Pandoc's guard against currency-like spans. `MathMode::On` preserves backslashes before `[]()` so client-side renderers such as KaTeX can see TeX delimiters. `MathMode::Off` treats TeX delimiters as ordinary Markdown text.
