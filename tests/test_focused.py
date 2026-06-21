"Focused dialect/math/tagfilter behavior (ported from tests/focused.rs)."
from pathlib import Path

from xhtmlmd import to_xhtml

FIX = Path(__file__).parent / "fixtures"

def _norm(s): return "\n".join(l.rstrip() for l in s.splitlines()).strip()

def test_dialect_fixture():
    md = (FIX / "dialect.md").read_text()
    expected = (FIX / "dialect.xhtml").read_text()
    assert _norm(to_xhtml(md)) == _norm(expected)

def test_default_math_mode_is_brackets():
    html = to_xhtml("\\(y\\)\n\n\\[\nx^2\n\\]\n\n$x$")
    assert '<span class="math inline">y</span>' in html
    assert '<div class="math display">x^2</div>' in html
    assert "<p>$x$</p>" in html

def test_math_modes_are_explicit():
    html = to_xhtml("$x$ and \\(y\\)", math="off")
    assert 'class="math' not in html
    assert "$x$ and (y)" in html
    html = to_xhtml(r"$x$ and \(y\) and \[z\]", math="on")
    assert 'class="math' not in html
    assert html == "<p>$x$ and \\(y\\) and \\[z\\]</p>\n"
    assert to_xhtml("\\[\nx^2\n\\]\n", math="on") == "<p>\\[\nx^2\n\\]</p>\n"
    html = to_xhtml("$x$ and \\(y\\)", math="brackets")
    assert "$x$" in html
    assert '<span class="math inline">y</span>' in html
    assert '<span class="math inline">x</span>' in to_xhtml("$x$", math="dollars")

def test_tagfilter_is_opt_in():
    inp = "No <textarea>.\n\n<script>alert(1)</script>"
    default = to_xhtml(inp)
    assert "<textarea>" in default
    assert "<script>" in default
    filtered = to_xhtml(inp, tagfilter=True)
    assert "&lt;textarea>" in filtered
    assert "&lt;script>" in filtered
    assert "&lt;/script>" in filtered
