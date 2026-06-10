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


def test_bracket_display_math_block():
    src = "\\[\nx^2\n\\]\n"
    assert to_xhtml(src, math="brackets") == '<div class="math display">x^2</div>\n'
    assert to_xhtml(src, math="on") == "<p>\\[\nx^2\n\\]</p>\n"


def test_invalid_math_mode_raises():
    with pytest.raises(ValueError, match="math must be"): to_xhtml("x", math="inline")


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
