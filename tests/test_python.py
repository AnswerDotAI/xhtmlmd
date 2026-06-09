import subprocess

import pytest

from xhtml_md_parser import render, to_xhtml


def test_to_xhtml_renders_markdown():
    assert to_xhtml("# Hello") == "<h1>Hello</h1>\n"


def test_render_alias(): assert render("*hi*") == "<p><em>hi</em></p>\n"


def test_math_mode_option():
    assert to_xhtml("$x$", math="off") == "<p>$x$</p>\n"
    assert to_xhtml("$x$", math="dollars") == '<p><span class="math inline">x</span></p>\n'


def test_invalid_math_mode_raises():
    with pytest.raises(ValueError, match="math must be"): to_xhtml("x", math="inline")


def test_cli_reads_markdown_from_stdin():
    res = subprocess.run(["xhtml-md"], input="# Hello\n", text=True, capture_output=True, check=True)
    assert res.stdout == "<h1>Hello</h1>\n"
    assert res.stderr == ""
