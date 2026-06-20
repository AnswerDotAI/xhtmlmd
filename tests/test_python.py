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
