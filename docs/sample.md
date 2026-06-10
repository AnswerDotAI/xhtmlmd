# xhtmlmd feature sample

This page is written as a tour of the Markdown features supported by xhtmlmd.
Each section starts with the Markdown source, then shows the same source rendered.

## Headings, paragraphs, and inline formatting

`````markdown
### Project notes {#project-notes .section-title}

This paragraph uses *emphasis*, **strong emphasis**, `inline code`,
~~deleted text~~, ==highlighted text==, E=mc^2^, and H~2~O.
`````

### Project notes {#project-notes .section-title}

This paragraph uses *emphasis*, **strong emphasis**, `inline code`,
~~deleted text~~, ==highlighted text==, E=mc^2^, and H~2~O.

## Links, images, and autolinks

`````markdown
[fast.ai](https://www.fast.ai/){.external rel="nofollow"} is a normal link
with attributes.

Bare links such as https://example.org/docs are linked automatically.
Angle links work too: <https://example.com/spec>.

![A small placeholder image](https://dummyimage.com/96x48/eeeeee/333333.png&text=demo){.thumbnail width="96" height="48"}
`````

[fast.ai](https://www.fast.ai/){.external rel="nofollow"} is a normal link
with attributes.

Bare links such as https://example.org/docs are linked automatically.
Angle links work too: <https://example.com/spec>.

![A small placeholder image](https://dummyimage.com/96x48/eeeeee/333333.png&text=demo){.thumbnail width="96" height="48"}

## Lists and tasks

`````markdown
- Write the outline
- Check the examples
  - Keep them short
  - Keep them readable

1. Parse the Markdown
2. Render XHTML
3. Add a stylesheet

- [x] Tables
- [x] Footnotes
- [ ] Final polish
`````

- Write the outline
- Check the examples
  - Keep them short
  - Keep them readable

1. Parse the Markdown
2. Render XHTML
3. Add a stylesheet

- [x] Tables
- [x] Footnotes
- [ ] Final polish

## Block quotes and rules

`````markdown
> A block quote can contain normal inline Markdown.
>
> - It can also contain lists.
> - This is useful for callouts and quoted notes.

`````

> A block quote can contain normal inline Markdown.
>
> - It can also contain lists.
> - This is useful for callouts and quoted notes.

## Tables

`````markdown
| Feature | Status | Notes |
|:--------|:------:|------:|
| Tables | ready | aligned columns |
| Math | ready | brackets mode |
| HTML | ready | raw or markdown-enabled |
`````

| Feature | Status | Notes |
|:--------|:------:|------:|
| Tables | ready | aligned columns |
| Math | ready | brackets mode |
| HTML | ready | raw or markdown-enabled |

## Code

`````markdown
Inline code uses backticks, as in `Options::default()`.

``` rust {#example-code .numberLines startFrom="10"}
fn main() {
    println!("hello from xhtmlmd");
}
```

    let indented_code = true;
`````

Inline code uses backticks, as in `Options::default()`.

``` rust {#example-code .numberLines startFrom="10"}
fn main() {
    println!("hello from xhtmlmd");
}
```

    let indented_code = true;

## Math in brackets mode

`````markdown
Inline math uses TeX parentheses: \(a^2 + b^2 = c^2\).

Display math uses TeX brackets:

\[
\int_0^1 x^2\,dx = \frac{1}{3}
\]
`````

Inline math uses TeX parentheses: \(a^2 + b^2 = c^2\).

Display math uses TeX brackets:

\[
\int_0^1 x^2\,dx = \frac{1}{3}
\]

## Attributes and spans

`````markdown
This paragraph gets attributes from the following block IAL.
{: #important-note .lead data-kind="sample"}

{:reusable: .note data-role="demo"}
This paragraph uses a named attribute list.
{: reusable #named-attribute-example}

Bracketed spans work too: [small but important]{.small .important}.
Code spans can have attributes: `render()`{.api-call}.
`````

This paragraph gets attributes from the following block IAL.
{: #important-note .lead data-kind="sample"}

{:reusable: .note data-role="demo"}
This paragraph uses a named attribute list.
{: reusable #named-attribute-example}

Bracketed spans work too: [small but important]{.small .important}.
Code spans can have attributes: `render()`{.api-call}.

## Definition lists

`````markdown
xhtmlmd
: A Markdown parser that renders XHTML fragments.

brackets math
: Math mode that recognizes `\(...\)` and `\[...\]`.

on math
: Math mode that preserves TeX delimiters for client-side renderers.

fenced div
: A Pandoc-style block container opened with colons.
`````

xhtmlmd
: A Markdown parser that renders XHTML fragments.

brackets math
: Math mode that recognizes `\(...\)` and `\[...\]`.

on math
: Math mode that preserves TeX delimiters for client-side renderers.

fenced div
: A Pandoc-style block container opened with colons.

## Footnotes

`````markdown
A short note can point to a footnote.[^sample-note]

[^sample-note]: Footnotes can contain *inline Markdown* and links such as
    <https://example.org/>.
`````

A short note can point to a footnote.[^sample-note]

[^sample-note]: Footnotes can contain *inline Markdown* and links such as
    <https://example.org/>.

## Abbreviations

`````markdown
The HTML5 standard changed the web.

*[HTML5]: HyperText Markup Language, version 5
`````

The HTML5 standard changed the web.

*[HTML5]: HyperText Markup Language, version 5

## Fenced divs

`````markdown
::: {#tip-box .callout .tip kind="tip"}
### A fenced div

Fenced divs are useful for notes, cards, columns, and other styled sections.
They can contain normal **Markdown**.
:::
`````

::: {#tip-box .callout .tip kind="tip"}
### A fenced div

Fenced divs are useful for notes, cards, columns, and other styled sections.
They can contain normal **Markdown**.
:::

## Raw HTML

`````markdown
<section class="raw-panel">
<h3>Raw HTML section</h3>

<p>Raw HTML can stay open across blank lines until its matching close tag.</p>
</section>
`````

<section class="raw-panel">
<h3>Raw HTML section</h3>

<p>Raw HTML can stay open across blank lines until its matching close tag.</p>
</section>

## Markdown inside HTML

`````markdown
<div markdown="1" class="markdown-panel">

### Markdown parsed inside HTML

- **Bold list item**
- `Code span`
- Inline math: \(x + y\)

</div>
`````

<div markdown="1" class="markdown-panel">

### Markdown parsed inside HTML

- **Bold list item**
- `Code span`
- Inline math: \(x + y\)

</div>
