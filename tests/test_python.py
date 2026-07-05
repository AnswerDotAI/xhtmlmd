import subprocess

import pytest

from xhtmlmd import render, to_xhtml


def test_to_xhtml_renders_markdown():
    assert to_xhtml("# Hello") == "<h1>Hello</h1>\n"


def test_render_alias(): assert render("*hi*") == "<p><em>hi</em></p>\n"


def test_math_mode_option():
    assert to_xhtml(r"\(x\)") == '<p><span class="math inline">x</span></p>\n'
    assert to_xhtml("$x$", math="off") == "<p>$x$</p>\n"
    assert to_xhtml(r"\(x\)", math="on") == "<p>\\(x\\)</p>\n"
    assert to_xhtml("$x$", math="dollars") == '<p><span class="math inline">x</span></p>\n'


def test_escaped_bracket_math_opener_is_literal_in_all_modes():
    assert to_xhtml(r"\\[", math="off") == "<p>\\[</p>\n"
    assert to_xhtml(r"\\[", math="on") == "<p>\\[</p>\n"
    assert to_xhtml(r"\\[", math="brackets") == "<p>\\[</p>\n"
    assert to_xhtml(r"\\[", math="dollars") == "<p>\\[</p>\n"


def test_bracket_display_math_block():
    src = "\\[\nx^2\n\\]\n"
    assert to_xhtml(src, math="brackets") == '<div class="math display">x^2</div>\n'
    assert to_xhtml(src, math="on") == "<p>\\[\nx^2\n\\]</p>\n"


def test_invalid_math_mode_raises():
    with pytest.raises(ValueError, match="math must be"): to_xhtml("x", math="inline")


def test_node_callback_can_override_heading():
    calls = []

    def heading(node, default_html):
        calls.append((node["type"], node["level"], default_html))
        return '<h1 data-hook="yes">Hooked</h1>\n'

    assert to_xhtml("# Hello", callbacks={"heading": heading}) == '<h1 data-hook="yes">Hooked</h1>\n'
    assert calls == [("heading", 1, "<h1>Hello</h1>\n")]


def test_node_callback_can_override_inline_code():
    def code(node, default_html):
        assert node["text"] == "x < y"
        assert default_html == "<code>x &lt; y</code>"
        return "<kbd>x &lt; y</kbd>"

    assert to_xhtml("Use `x < y`.", callbacks={"code": code}) == "<p>Use <kbd>x &lt; y</kbd>.</p>\n"


def test_code_block_callback_can_return_fastpylight_node():
    from fastpylight import highlight

    def highlight_code(node, default_html):
        assert node["type"] == "code_block"
        assert node["lang"] == "python"
        assert default_html == '<pre><code class="language-python">if x:\n    return 1\n</code></pre>\n'
        return highlight(node["text"], node["lang"]) + "\n"

    html = to_xhtml("```python\nif x:\n    return 1\n```\n", callbacks={"code_block": highlight_code})
    assert html.startswith("<hl-code toks=")
    assert "<pre><code>if x:\n    return 1\n</code></pre></hl-code>\n" in html


def test_image_callback_node_has_alt_text():
    def image(node, default_html):
        assert node["alt"] == "Bold pic"
        assert node["url"] == "pic.png"
        assert node["title"] == "ttl"
        return None

    html = to_xhtml('![**Bold** pic](pic.png "ttl")', callbacks={"image": image})
    assert '<img src="pic.png" alt="Bold pic" title="ttl" />' in html


def test_math_callbacks_with_math_core():
    from math_core import LatexToMathML

    mathml = LatexToMathML()

    def render_math(node, default_html):
        html = mathml.convert_with_local_counter(node["tex"], displaystyle=node["type"] == "math_block")
        return html + ("\n" if node["type"] == "math_block" else "")

    callbacks = {"math_inline": render_math, "math_block": render_math}
    assert to_xhtml(r"Inline \(x^2\).", callbacks=callbacks) == "<p>Inline <math><msup><mi>x</mi><mn>2</mn></msup></math>.</p>\n"
    assert to_xhtml("\\[\n\\frac{a}{b}\n\\]\n", callbacks=callbacks) == '<math display="block"><mfrac><mi>a</mi><mi>b</mi></mfrac></math>\n'
    assert to_xhtml("$x^2$", callbacks=callbacks) == "<p>$x^2$</p>\n"
    assert to_xhtml("$x^2$", math="dollars", callbacks=callbacks) == "<p><math><msup><mi>x</mi><mn>2</mn></msup></math></p>\n"


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
    assert to_xhtml(src) == '<p id="id" class="lead">Para with <a href="/url">ref</a>.</p>\n'
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


def test_cli_reads_markdown_from_stdin():
    res = subprocess.run(["xhtmlmd"], input="# Hello\n", text=True, capture_output=True, check=True)
    assert res.stdout == "<h1>Hello</h1>\n"
    assert res.stderr == ""


def test_cli_defaults_to_bracket_math():
    res = subprocess.run(["xhtmlmd"], input="\\[\nx^2\n\\]\n", text=True, capture_output=True, check=True)
    assert res.stdout == '<div class="math display">x^2</div>\n'
    assert res.stderr == ""


def test_cli_math_on_preserves_katex_delimiters():
    res = subprocess.run(["xhtmlmd", "--math=on"], input="\\[\nx^2\n\\]\n", text=True, capture_output=True, check=True)
    assert res.stdout == "<p>\\[\nx^2\n\\]</p>\n"
    assert res.stderr == ""

def test_max_link_paren_depth_is_honored():
    deep = "[a](" + "(" * 40 + "x" + ")" * 40 + ")"
    assert "<a" not in to_xhtml(deep)  # over the default cap of 32
    assert "<a" in to_xhtml(deep, max_link_paren_depth=64)
    shallow = "[a](((x)))"
    assert "<a" in to_xhtml(shallow)
    assert "<a" not in to_xhtml(shallow, max_link_paren_depth=1)
