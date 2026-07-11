import subprocess

import pytest

from xhtmlmd import render, to_xhtml
from test_conformance import normalize_html


def assert_html(actual, expected): assert normalize_html(actual) == normalize_html(expected)


def test_to_xhtml_renders_markdown():
    assert_html(to_xhtml("# Hello"), "<h1>Hello</h1>")


def test_render_alias(): assert_html(render("*hi*"), "<p><em>hi</em></p>")


def test_math_mode_option():
    assert_html(to_xhtml(r"\(x\)"), '<p><span class="math inline">x</span></p>')
    assert_html(to_xhtml("$x$", math="off"), "<p>$x$</p>")
    assert_html(to_xhtml(r"\(x\)", math="on"), "<p>\\(x\\)</p>")
    assert_html(to_xhtml("$x$", math="dollars"), '<p><span class="math inline">x</span></p>')


def test_escaped_bracket_math_opener_is_literal_in_all_modes():
    for mode in ("off", "on", "brackets", "dollars"): assert_html(to_xhtml(r"\\[", math=mode), "<p>\\[</p>")


def test_bracket_display_math_block():
    src = "\\[\nx^2\n\\]\n"
    assert_html(to_xhtml(src, math="brackets"), '<div class="math display">x^2</div>')
    assert_html(to_xhtml(src, math="on"), "<p>\\[\nx^2\n\\]</p>")


def test_invalid_math_mode_raises():
    with pytest.raises(ValueError, match="math must be"): to_xhtml("x", math="inline")


def test_node_callback_can_override_heading():
    calls = []

    def heading(node, default_html):
        calls.append((node["type"], node["level"], default_html))
        return '<h1 data-hook="yes">Hooked</h1>\n'

    assert_html(to_xhtml("# Hello", callbacks={"heading": heading}), '<h1 data-hook="yes">Hooked</h1>')
    assert len(calls) == 1
    assert calls[0][:2] == ("heading", 1)
    assert_html(calls[0][2], "<h1>Hello</h1>")


def test_node_callback_can_override_inline_code():
    def code(node, default_html):
        assert node["text"] == "x < y"
        assert_html(default_html, "<code>x &lt; y</code>")
        return "<kbd>x &lt; y</kbd>"

    assert_html(to_xhtml("Use `x < y`.", callbacks={"code": code}), "<p>Use <kbd>x &lt; y</kbd>.</p>")


def test_code_block_callback_can_return_fastpylight_node():
    from fastpylight import highlight

    def highlight_code(node, default_html):
        assert node["type"] == "code_block"
        assert node["lang"] == "python"
        assert_html(default_html, '<pre><code class="language-python">if x:\n    return 1\n</code></pre>')
        return highlight(node["text"], node["lang"]) + "\n"

    html = to_xhtml("```python\nif x:\n    return 1\n```\n", callbacks={"code_block": highlight_code})
    assert html.startswith("<hl-code toks=")
    assert "<pre><code>if x:\n    return 1\n</code></pre></hl-code>\n" in html


def test_image_callback_node_has_alt_text():
    calls = []

    def image(node, default_html):
        calls.append(node)
        assert node["alt"] == "Bold pic"
        assert node["url"] == "pic.png"
        assert node["title"] == "ttl"
        return None

    html = to_xhtml('![**Bold** pic](pic.png "ttl")', callbacks={"image": image})
    assert len(calls) == 1
    assert '<img src="pic.png" alt="Bold pic" title="ttl" />' in html


def test_math_callbacks_with_math_core():
    from math_core import LatexToMathML

    mathml = LatexToMathML()

    def render_math(node, default_html):
        html = mathml.convert_with_local_state(node["tex"], displaystyle=node["type"] == "math_block")
        return html + ("\n" if node["type"] == "math_block" else "")

    callbacks = {"math_inline": render_math, "math_block": render_math}
    assert_html(to_xhtml(r"Inline \(x^2\).", callbacks=callbacks), "<p>Inline <math><msup><mi>x</mi><mn>2</mn></msup></math>.</p>")
    assert_html(to_xhtml("\\[\n\\frac{a}{b}\n\\]\n", callbacks=callbacks), '<math display="block"><mfrac><mi>a</mi><mi>b</mi></mfrac></math>')
    assert_html(to_xhtml("$x^2$", callbacks=callbacks), "<p>$x^2$</p>")
    assert_html(to_xhtml("$x^2$", math="dollars", callbacks=callbacks), "<p><math><msup><mi>x</mi><mn>2</mn></msup></math></p>")


def test_blocks_top_level_source_spans():
    from xhtmlmd import blocks
    src = ("# Title\n\nSome para\nover two lines.\n\n```python\nx = 1\n```\n\n"
        "- a list\n- items\n\n[ref]: https://x.com\n\nTail para with [ref].\n")
    bs = blocks(src)
    assert [b["type"] for b in bs] == ["heading", "paragraph", "code_block", "list", "link_ref", "paragraph"]
    lines = src.split("\n")
    slices = ["\n".join(lines[b["start"]:b["end"]]) for b in bs]
    assert slices[0] == "# Title"
    assert slices[1] == "Some para\nover two lines."
    assert slices[2] == "```python\nx = 1\n```"
    assert slices[3] == "- a list\n- items"
    assert bs[2]["lang"] == "python" and bs[2]["text"] == "x = 1\n"
    covered = {i for b in bs for i in range(b["start"], b["end"])}
    assert all(i in covered for i, l in enumerate(lines) if l.strip())


def test_blocks_span_edge_cases():
    from xhtmlmd import blocks
    src = "Setext\n======\n\nhead | er\n---- | --\ncell | s\n\n[^n]: a note def\n\n<div>\nraw\n</div>\n"
    bs = blocks(src)
    assert [b["type"] for b in bs] == ["heading", "table", "footnote_def", "html_block"]
    lines = src.split("\n")
    assert "\n".join(lines[bs[1]["start"]:bs[1]["end"]]) == "head | er\n---- | --\ncell | s"
    assert blocks("") == []


def test_blocks_keep_pending_ial_with_next_block():
    from xhtmlmd import blocks, to_xhtml
    src = "[ref]: /url\n{: #id .lead}\nPara with [ref].\n"
    assert_html(to_xhtml(src), '<p id="id" class="lead">Para with <a href="/url">ref</a>.</p>')
    bs = blocks(src)
    lines = src.split("\n")
    slices = ["\n".join(lines[b["start"]:b["end"]]) for b in bs]
    assert [b["type"] for b in bs] == ["link_ref", "paragraph"]
    assert slices == ["[ref]: /url", "{: #id .lead}\nPara with [ref]."]


def test_blocks_keep_pending_ial_after_non_attr_spans():
    from xhtmlmd import blocks, to_xhtml
    cases = [
        ("[^n]: note\n{: #id}\nPara\n", ["footnote_def", "paragraph"], ["[^n]: note", "{: #id}\nPara"]),
        ("<div>\nraw\n</div>\n{: #id}\nPara\n", ["html_block", "paragraph"], ["<div>\nraw\n</div>", "{: #id}\nPara"])]
    for src, types, slices in cases:
        assert '<p id="id">Para</p>' in to_xhtml(src)
        lines = src.split("\n")
        bs = blocks(src)
        assert [b["type"] for b in bs] == types
        assert ["\n".join(lines[b["start"]:b["end"]]) for b in bs] == slices


def test_blocks_ial_never_leapfrogs_non_attr_spans():
    from xhtmlmd import blocks, to_xhtml
    src = "Para\n\n<div>\nraw\n</div>\n{: #id}\nTail\n"
    assert '<p id="id">Tail</p>' in to_xhtml(src)
    lines = src.split("\n")
    bs = blocks(src)
    assert [b["type"] for b in bs] == ["paragraph", "html_block", "paragraph"]
    assert ["\n".join(lines[b["start"]:b["end"]]) for b in bs] == ["Para", "<div>\nraw\n</div>", "{: #id}\nTail"]
    for src in [src, "{: #id}\n<div>\nraw\n</div>\n\nPara\n", "Para\n\n[a]: /u\n{: .x}\nTail [a]\n"]:
        bs = blocks(src)
        for a, b in zip(bs, bs[1:]): assert a["end"] <= b["start"], (src, bs)


def test_rewrite_inline_constructs_and_callback_data():
    from xhtmlmd import rewrite
    seen = []

    def image(node):
        seen.append(node)
        return {"url": "images/plot.png"}

    def math(node):
        seen.append(node)
        return rf"\({node['tex']}\)"

    src = 'Before ![plot](data:image/png;base64,eA== "Chart") and $x^2$ after.'
    got = rewrite(src, {"image": image, "math_inline": math}, math="dollars")
    assert got == 'Before ![plot](images/plot.png "Chart") and \\(x^2\\) after.'
    assert seen == [
        dict(type="image", form="inline", source='![plot](data:image/png;base64,eA== "Chart")', start=7, end=50,
            alt="plot", url="data:image/png;base64,eA==", title="Chart"),
        dict(type="math_inline", source="$x^2$", start=55, end=60, delimiter="$", display=False, tex="x^2")]


def test_rewrite_skips_code_and_fenced_blocks():
    from xhtmlmd import rewrite
    src = "`$code$ ![x](bad)` [label](https://x/$url$) <i data-x='$html$'> and $math$\n\n- before\n  ```\n  $fenced$ ![x](bad)\n  ```\n- ![x](data:x)\n"
    callbacks = {"image": lambda node: {"url": "ok"}, "math_inline": lambda node: rf"\({node['tex']}\)"}
    got = rewrite(src, callbacks, math="dollars")
    assert got == "`$code$ ![x](bad)` [label](https://x/$url$) <i data-x='$html$'> and \\(math\\)\n\n- before\n  ```\n  $fenced$ ![x](bad)\n  ```\n- ![x](ok)\n"


def test_rewrite_none_unknown_components_and_crlf():
    from xhtmlmd import rewrite
    src = "![x](old)\r\n$x$\r\n"
    assert rewrite(src, {"image": lambda node: None}, math="dollars") == src
    with pytest.raises(ValueError, match="unknown image replacement field"):
        rewrite(src, {"image": lambda node: {"nonsense": "y"}}, math="dollars")


def test_rewrite_unicode_component_edits():
    from xhtmlmd import rewrite
    seen = []
    src = "é $x$ ![x](old)\r\n"
    callbacks = {"image": lambda node: seen.append(node) or {"url": "new"}, "math_inline": lambda node: {"tex": "y"}}
    got = rewrite(src, callbacks, math="dollars")
    assert got == "é $y$ ![x](new)\r\n"
    assert [(node["source"], node["start"], node["end"]) for node in seen] == [("![x](old)", 6, 15)]


def test_cli_reads_markdown_from_stdin():
    res = subprocess.run(["xhtmlmd"], input="# Hello\n", text=True, capture_output=True, check=True)
    assert_html(res.stdout, "<h1>Hello</h1>")
    assert res.stderr == ""


def test_cli_defaults_to_bracket_math():
    res = subprocess.run(["xhtmlmd"], input="\\[\nx^2\n\\]\n", text=True, capture_output=True, check=True)
    assert_html(res.stdout, '<div class="math display">x^2</div>')
    assert res.stderr == ""


def test_cli_math_on_preserves_katex_delimiters():
    res = subprocess.run(["xhtmlmd", "--math=on"], input="\\[\nx^2\n\\]\n", text=True, capture_output=True, check=True)
    assert_html(res.stdout, "<p>\\[\nx^2\n\\]</p>")
    assert res.stderr == ""

def test_max_link_paren_depth_is_honored():
    deep = "[a](" + "(" * 40 + "x" + ")" * 40 + ")"
    assert "<a" not in to_xhtml(deep)  # over the default cap of 32
    assert "<a" in to_xhtml(deep, max_link_paren_depth=64)
    shallow = "[a](((x)))"
    assert "<a" in to_xhtml(shallow)
    assert "<a" not in to_xhtml(shallow, max_link_paren_depth=1)
